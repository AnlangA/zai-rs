//! Atomic writer for an already-buffered response body.
//!
//! [`atomic_download`] writes a [`bytes::Bytes`] value into a same-directory
//! temporary `.part` file (Unix mode 0600), flushes and syncs it, closes the
//! handle, then renames it to the target. The response is already buffered before
//! this function is called; this module does not stream network data.
//!
//! A pre-existing destination is refused. Failure or cancellation drops the
//! guard and attempts to delete the partial file.

use std::path::{Path, PathBuf};

use crate::{ZaiError, ZaiResult, client::error::codes};

/// Atomically write buffered `body` bytes to `dest`.
///
/// Returns an error when the target already exists, the temporary file cannot
/// be created or written, or the final rename fails.
pub async fn atomic_download(dest: &Path, body: bytes::Bytes) -> ZaiResult<()> {
    if dest.exists() {
        return Err(ZaiError::FileError {
            // There is no dedicated existing-target SDK code; retain the generic
            // file-category code used by this helper.
            code: codes::SDK_FILE_NOT_FOUND,
            message: format!("download target already exists: {}", dest.display()),
        });
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
        // Random-ish name in the same directory; create_new refuses an existing
        // path. Unix mode 0600 via the builder on unix.
        let name = format!(".zai-dl-{}-{}.part", std::process::id(), nanos());
        let path = dir.join(name);
        let mut builder = tokio::fs::OpenOptions::new();
        builder.create_new(true).write(true).read(false);
        #[cfg(unix)]
        {
            // tokio::fs::OpenOptions proxies std::os::unix::fs::OpenOptionsExt;
            // the trait must be in scope for `.mode(0o600)`.
            #[allow(unused_imports)]
            use std::os::unix::fs::OpenOptionsExt;
            builder.mode(0o600);
        }
        let file = builder.open(&path).await.map_err(ZaiError::from)?;
        Ok(Self {
            file: Some(file),
            path,
            committed: false,
        })
    }

    async fn write_all(&mut self, body: &[u8]) -> ZaiResult<()> {
        use tokio::io::AsyncWriteExt;
        let file = self.file.as_mut().expect("partial file open");
        file.write_all(body).await.map_err(ZaiError::from)?;
        Ok(())
    }

    async fn commit(mut self, dest: &Path) -> ZaiResult<()> {
        use tokio::io::AsyncWriteExt;
        let mut file = self.file.take().expect("partial file present");
        file.flush().await.map_err(ZaiError::from)?;
        file.sync_all().await.map_err(ZaiError::from)?;
        drop(file); // close before rename
        tokio::fs::rename(&self.path, dest)
            .await
            .map_err(ZaiError::from)?;
        self.committed = true;
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

fn nanos() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0)
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
    async fn atomic_download_writes_and_renames() {
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
}
