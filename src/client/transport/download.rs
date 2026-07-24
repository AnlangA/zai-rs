//! Atomic writer for a streamed file response.
//!
//! [`AtomicDownload`] writes each incoming chunk into a same-directory private
//! `.part` file (Unix mode 0600), flushes and syncs it, closes the handle, then
//! atomically links it to the target without replacing an existing path.
//!
//! A pre-existing destination is refused. Failure or cancellation drops the
//! guard and attempts to delete the partial file.

use std::path::{Path, PathBuf};

use crate::{ZaiError, ZaiResult, client::error::codes};

/// Same-directory streaming writer that publishes only a complete file.
pub(crate) struct AtomicDownload {
    destination: PathBuf,
    partial: PartialFile,
    length: usize,
}

impl AtomicDownload {
    /// Reserve a private partial file for `destination`.
    ///
    /// This performs an early destination check for fast failure. The final
    /// hard-link publication checks again so a concurrent creator is never
    /// overwritten.
    pub(crate) async fn new(destination: &Path) -> ZaiResult<Self> {
        if tokio::fs::try_exists(destination)
            .await
            .map_err(ZaiError::from)?
        {
            return Err(target_exists(destination));
        }
        let parent = destination
            .parent()
            .ok_or_else(|| invalid("download target has no parent directory"))?;
        Ok(Self {
            destination: destination.to_path_buf(),
            partial: PartialFile::new(parent).await?,
            length: 0,
        })
    }

    /// Append one response chunk without retaining it after the write.
    pub(crate) async fn write_chunk(&mut self, chunk: &[u8]) -> ZaiResult<()> {
        self.partial.write_all(chunk).await?;
        self.length = self
            .length
            .checked_add(chunk.len())
            .ok_or_else(|| invalid("download length overflow"))?;
        Ok(())
    }

    /// Flush, sync and atomically publish the completed partial file.
    pub(crate) async fn commit(self) -> ZaiResult<usize> {
        let length = self.length;
        self.partial.commit(&self.destination).await?;
        Ok(length)
    }
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
        let dir = dir.to_path_buf();
        // The blocking task returns an armed guard, not a bare file. If the
        // awaiting download future is cancelled at any point, either the task
        // or its undelivered output drops that guard and removes the path.
        tokio::task::spawn_blocking(move || Self::new_blocking(&dir))
            .await
            .map_err(|_| invalid("partial-file creation task failed"))?
    }

    fn new_blocking(dir: &Path) -> ZaiResult<Self> {
        // Retry the extremely unlikely random-name collision instead of
        // surfacing it as an unrelated I/O failure.
        for _ in 0..8 {
            let name = format!(
                ".zai-dl-{}-{:016x}.part",
                std::process::id(),
                fastrand::u64(..)
            );
            let path = dir.join(name);
            let mut builder = std::fs::OpenOptions::new();
            builder.create_new(true).write(true).read(false);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                builder.mode(0o600);
            }
            match builder.open(&path) {
                Ok(file) => {
                    return Ok(Self {
                        file: Some(tokio::fs::File::from_std(file)),
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
            // Close first: Windows refuses to unlink an open file, while Unix
            // permits it. Taking the handle keeps cancellation cleanup
            // portable.
            drop(self.file.take());
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

    async fn atomic_download(dest: &Path, body: bytes::Bytes) -> ZaiResult<usize> {
        let mut download = AtomicDownload::new(dest).await?;
        download.write_chunk(&body).await?;
        download.commit().await
    }

    #[tokio::test]
    async fn atomic_download_writes_without_partial_residue() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("out.bin");
        let body = bytes::Bytes::from_static(b"hello world");
        assert_eq!(
            atomic_download(&dest, body.clone()).await.unwrap(),
            body.len()
        );
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
