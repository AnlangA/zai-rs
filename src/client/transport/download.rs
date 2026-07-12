//! Atomic writer for an already-buffered response body.
//!
//! [`atomic_download`] writes a [`bytes::Bytes`] value into a same-directory
//! temporary `.part` file (Unix mode 0600), flushes and syncs it, closes the
//! handle, then atomically links it to the target without replacing an existing
//! path. The response is already buffered before this function is called; this
//! module does not stream network data.
//!
//! A pre-existing destination is refused. Failure or cancellation drops the
//! guard and attempts to delete the partial file.

use std::path::{Path, PathBuf};

use crate::{ZaiError, ZaiResult, client::error::codes};

/// Atomically write buffered `body` bytes to `dest`.
///
/// Returns an error when the target already exists, the temporary file cannot
/// be created or written, or the final no-replace link fails.
pub async fn atomic_download(dest: &Path, body: bytes::Bytes) -> ZaiResult<()> {
    if tokio::fs::try_exists(dest).await.map_err(ZaiError::from)? {
        return Err(target_exists(dest));
    }
    let parent = dest
        .parent()
        .ok_or_else(|| invalid("download target has no parent directory"))?;
    let mut partial = PartialFile::new(parent).await?;
    partial.write_all(&body).await?;
    partial.commit(dest).await?;
    Ok(())
}

/// A partial-file guard: owns a `(File, PathBuf)` and deletes the partial on
/// Drop unless `commit` consumed it.
struct PartialFile {
    file: Option<tokio::fs::File>,
    path: PathBuf,
    committed: bool,
}

impl PartialFile {
    async fn new(dir: &Path) -> ZaiResult<Self> {
        // Retry the extremely unlikely random-name collision instead of
        // surfacing it as an unrelated I/O failure.
        for _ in 0..8 {
            let name = format!(
                ".zai-dl-{}-{:016x}.part",
                std::process::id(),
                fastrand::u64(..)
            );
            let path = dir.join(name);
            let mut builder = tokio::fs::OpenOptions::new();
            builder.create_new(true).write(true).read(false);
            #[cfg(unix)]
            builder.mode(0o600);
            match builder.open(&path).await {
                Ok(file) => {
                    return Ok(Self {
                        file: Some(file),
                        path,
                        committed: false,
                    });
                },
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(error.into()),
            }
        }
        Err(invalid("could not allocate a unique partial-file name"))
    }

    async fn write_all(&mut self, body: &[u8]) -> ZaiResult<()> {
        use tokio::io::AsyncWriteExt;
        let file = self
            .file
            .as_mut()
            .ok_or_else(|| invalid("partial file is not open"))?;
        file.write_all(body).await.map_err(ZaiError::from)?;
        Ok(())
    }

    async fn commit(mut self, dest: &Path) -> ZaiResult<()> {
        use tokio::io::AsyncWriteExt;
        let mut file = self
            .file
            .take()
            .ok_or_else(|| invalid("partial file is not open"))?;
        file.flush().await.map_err(ZaiError::from)?;
        file.sync_all().await.map_err(ZaiError::from)?;
        drop(file); // close before publishing the destination link
        match tokio::fs::hard_link(&self.path, dest).await {
            Ok(()) => {},
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                return Err(target_exists(dest));
            },
            Err(error) => return Err(error.into()),
        }
        // Removing the temporary name cannot invalidate the destination hard
        // link. Drop retries synchronously if this best-effort async removal
        // fails (for example, during runtime shutdown).
        if tokio::fs::remove_file(&self.path).await.is_ok() {
            self.committed = true;
        }
        Ok(())
    }
}

impl Drop for PartialFile {
    fn drop(&mut self) {
        if !self.committed {
            // Best-effort synchronous removal; the async runtime may be gone.
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

fn target_exists(dest: &Path) -> ZaiError {
    ZaiError::FileError {
        code: codes::SDK_IO,
        message: format!("download target already exists: {}", dest.display()),
    }
}

fn invalid(msg: &str) -> ZaiError {
    ZaiError::ApiError {
        code: codes::SDK_CONFIG,
        message: msg.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn atomic_download_writes_without_partial_residue() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("out.bin");
        let body = bytes::Bytes::from_static(b"hello world");
        atomic_download(&dest, body.clone()).await.unwrap();
        let read = tokio::fs::read(&dest).await.unwrap();
        assert_eq!(read, body.as_ref());
        // No .part residue.
        let mut count = 0;
        for e in std::fs::read_dir(dir.path()).unwrap() {
            let e = e.unwrap();
            if e.file_name().to_string_lossy().ends_with(".part") {
                count += 1;
            }
        }
        assert_eq!(count, 0, ".part residue left behind");
    }

    #[tokio::test]
    async fn atomic_download_refuses_existing_target() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("out.bin");
        tokio::fs::write(&dest, b"prior").await.unwrap();
        let r = atomic_download(&dest, bytes::Bytes::from_static(b"new")).await;
        assert!(r.is_err(), "existing target must be refused");
        // Original content untouched.
        let read = tokio::fs::read(&dest).await.unwrap();
        assert_eq!(read, b"prior");
    }

    #[tokio::test]
    async fn concurrent_downloads_never_replace_the_winner() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("out.bin");
        let (first, second) = tokio::join!(
            atomic_download(&dest, bytes::Bytes::from_static(b"first")),
            atomic_download(&dest, bytes::Bytes::from_static(b"second")),
        );
        assert_ne!(first.is_ok(), second.is_ok(), "exactly one writer must win");
        let body = tokio::fs::read(&dest).await.unwrap();
        assert!(body == b"first" || body == b"second");
    }
}
