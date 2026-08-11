//! Atomic writer for a streamed file response.
//!
//! [`AtomicDownload`] writes each incoming chunk into a same-directory private
//! `.part` file (Unix mode 0600), flushes and syncs it, closes the handle, then
//! publishes it without replacing an existing path. Hard links are preferred;
//! filesystems that report them as unsupported use a platform-aware
//! no-clobber publication primitive. In every path the destination appears
//! only after the complete file has been synced.
//!
//! A pre-existing destination is refused. Failure or cancellation drops the
//! guard and attempts to delete the partial file.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::{ZaiError, ZaiResult, client::error::codes};

const DEFERRED_CLEANUP_MAX: usize = 8;
static DEFERRED_CLEANUP_BUDGET: CleanupBudget = CleanupBudget::new(DEFERRED_CLEANUP_MAX);

/// Same-directory streaming writer that publishes only a complete file.
pub(crate) struct AtomicDownload {
    destination: PathBuf,
    partial: PartialFile,
    directory_sync: DirectorySyncPlan,
    length: usize,
}

/// Directory entries that must be synced after publication for a successful
/// Unix download to retain the same durability guarantee when parent
/// directories were created for this operation.
#[derive(Debug, Clone)]
pub(crate) struct DirectorySyncPlan {
    #[cfg(unix)]
    directories: Vec<PathBuf>,
}

impl DirectorySyncPlan {
    /// Capture the directory chain before `create_dir_all` changes which
    /// ancestors predate this download.
    pub(crate) async fn capture_before_create(destination: &Path) -> ZaiResult<Self> {
        #[cfg(unix)]
        {
            let Some(parent) = destination.parent() else {
                return Ok(Self {
                    directories: Vec::new(),
                });
            };
            let mut cursor = parent.to_path_buf();
            let mut directories = Vec::new();

            loop {
                if !directories.contains(&cursor) {
                    directories.push(cursor.clone());
                }
                match tokio::fs::metadata(&cursor).await {
                    Ok(metadata) if metadata.is_dir() => break,
                    Ok(_) => {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::NotADirectory,
                            format!(
                                "download parent component is not a directory: {}",
                                cursor.display()
                            ),
                        )
                        .into());
                    },
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                        let Some(ancestor) = cursor.parent() else {
                            return Err(error.into());
                        };
                        cursor = ancestor.to_path_buf();
                    },
                    Err(error) => return Err(error.into()),
                }
            }

            Ok(Self { directories })
        }

        #[cfg(not(unix))]
        {
            let _ = destination;
            Ok(Self {})
        }
    }
}

impl AtomicDownload {
    /// Reserve a private partial file for `destination`.
    ///
    /// This performs an early destination check for fast failure. The final
    /// no-clobber publication checks again so a concurrent creator is never
    /// overwritten.
    pub(crate) async fn new(
        destination: &Path,
        directory_sync: DirectorySyncPlan,
    ) -> ZaiResult<Self> {
        if destination.as_os_str().is_empty() {
            return Err(invalid("download target must not be empty"));
        }
        // Resolve once so a process-wide CWD change during a long download
        // cannot send the private partial and final publication to different
        // directories. This also gives single-component paths a real parent.
        let destination = std::path::absolute(destination).map_err(ZaiError::from)?;
        if tokio::fs::try_exists(&destination)
            .await
            .map_err(ZaiError::from)?
        {
            return Err(target_exists(&destination));
        }
        let parent = destination
            .parent()
            .ok_or_else(|| invalid("download target has no parent directory"))?
            .to_path_buf();
        Ok(Self {
            destination,
            partial: PartialFile::new(&parent).await?,
            directory_sync,
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

    /// Flush, sync and publish the completed file without replacing a target.
    pub(crate) async fn commit(self) -> ZaiResult<usize> {
        #[cfg(unix)]
        {
            self.commit_with_directory_sync(sync_directory).await
        }

        #[cfg(not(unix))]
        {
            let Self {
                destination,
                partial,
                directory_sync: _,
                length,
            } = self;
            partial.commit(&destination).await?;
            Ok(length)
        }
    }

    #[cfg(unix)]
    async fn commit_with_directory_sync<F, Fut>(self, sync: F) -> ZaiResult<usize>
    where
        F: FnMut(PathBuf) -> Fut,
        Fut: std::future::Future<Output = std::io::Result<()>>,
    {
        let Self {
            destination,
            partial,
            directory_sync,
            length,
        } = self;
        partial.commit(&destination).await?;
        sync_directory_chain_with(&directory_sync, &destination, sync).await?;
        Ok(length)
    }
}

/// A partial-file guard. `cleanup_path == Some` means this value exclusively
/// owns deletion of that private name; publication fallback transfers that
/// ownership to a `TempPath` before entering a blocking task.
struct PartialFile {
    file: Option<tokio::fs::File>,
    cleanup_path: Option<PathBuf>,
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
                        cleanup_path: Some(path),
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
        self.publish_noclobber(dest).await?;
        Ok(())
    }

    async fn publish_noclobber(&mut self, dest: &Path) -> ZaiResult<()> {
        let partial = self
            .cleanup_path
            .as_ref()
            .ok_or_else(|| invalid("partial file has no cleanup path"))?;
        let result = tokio::fs::hard_link(partial, dest).await;
        self.finish_publication(dest, result).await
    }

    async fn finish_publication(
        &mut self,
        dest: &Path,
        hard_link_result: std::io::Result<()>,
    ) -> ZaiResult<()> {
        match hard_link_result {
            Ok(()) => {
                // Removing the temporary name cannot invalidate the destination
                // link. Drop retries through the bounded cleanup path if this
                // async removal fails.
                if let Some(path) = self.cleanup_path.as_ref()
                    && tokio::fs::remove_file(path).await.is_ok()
                {
                    self.cleanup_path = None;
                }
                Ok(())
            },
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                Err(target_exists(dest))
            },
            Err(error) if error.kind() == std::io::ErrorKind::Unsupported => {
                // Transfer the sole cleanup responsibility into the blocking
                // task. Cancellation can discard its JoinHandle, but the task's
                // TempPath still removes the private name on every failure.
                let path = self
                    .cleanup_path
                    .take()
                    .ok_or_else(|| invalid("partial file has no cleanup path"))?;
                match persist_path_noclobber(path, dest.to_path_buf()).await {
                    Ok(()) => Ok(()),
                    Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                        Err(target_exists(dest))
                    },
                    Err(error) => Err(error.into()),
                }
            },
            Err(error) => Err(error.into()),
        }
    }
}

impl Drop for PartialFile {
    fn drop(&mut self) {
        if let Some(path) = self.cleanup_path.take() {
            // Close first: Windows refuses to unlink an open file, while Unix
            // permits it. Taking the handle keeps cancellation cleanup
            // portable.
            drop(self.file.take());
            cleanup_partial(path);
        }
    }
}

/// Process-wide cap for best-effort cleanup work queued onto Tokio's blocking
/// pool. Saturation falls back to synchronous deletion instead of creating an
/// unbounded cleanup backlog.
struct CleanupBudget {
    active: AtomicUsize,
    max: usize,
}

impl CleanupBudget {
    const fn new(max: usize) -> Self {
        Self {
            active: AtomicUsize::new(0),
            max,
        }
    }

    fn try_acquire(&'static self) -> Option<CleanupPermit> {
        self.active
            .fetch_update(Ordering::AcqRel, Ordering::Relaxed, |active| {
                (active < self.max).then_some(active + 1)
            })
            .ok()
            .map(|_| CleanupPermit { budget: self })
    }
}

struct CleanupPermit {
    budget: &'static CleanupBudget,
}

impl Drop for CleanupPermit {
    fn drop(&mut self) {
        let previous = self.budget.active.fetch_sub(1, Ordering::AcqRel);
        debug_assert!(previous > 0, "deferred cleanup budget underflow");
    }
}

/// Owns both the delete-on-drop path guard and its bounded queue permit.
/// Dropping a queued closure during runtime shutdown still performs cleanup and
/// releases the permit; no task ever captures an unguarded `PathBuf`.
struct DeferredCleanup {
    path: Option<tempfile::TempPath>,
    _permit: CleanupPermit,
}

impl DeferredCleanup {
    fn close(mut self) {
        if let Some(path) = self.path.take() {
            let _ = path.close();
        }
    }
}

fn cleanup_partial(path: PathBuf) {
    // This path is absolute in every production call. Keep a fallback copy so
    // even an unexpected guard-construction error still gets one removal try.
    let fallback = path.clone();
    let guard = match tempfile::TempPath::try_from_path(path) {
        Ok(guard) => guard,
        Err(_) => {
            let _ = std::fs::remove_file(fallback);
            return;
        },
    };

    let Ok(handle) = tokio::runtime::Handle::try_current() else {
        let _ = guard.close();
        return;
    };
    let Some(permit) = DEFERRED_CLEANUP_BUDGET.try_acquire() else {
        let _ = guard.close();
        return;
    };
    let cleanup = DeferredCleanup {
        path: Some(guard),
        _permit: permit,
    };

    // Drop implementations must not unwind. The captured job remains the sole
    // cleanup owner even if scheduling panics: it is either dropped during
    // unwind or retained in Tokio's blocking queue until it runs or is
    // discarded. CleanupBudget—not Tokio's queue—bounds this cleanup class.
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        drop(handle.spawn_blocking(move || cleanup.close()));
    }));
}

async fn persist_path_noclobber(path: PathBuf, destination: PathBuf) -> std::io::Result<()> {
    // Arm cleanup before queuing the blocking task. If runtime shutdown drops
    // the queued closure before it starts, the captured guard still unlinks the
    // private name; a captured bare PathBuf would leak it.
    let path = tempfile::TempPath::try_from_path(path)?;
    tokio::task::spawn_blocking(move || {
        path.persist_noclobber(destination)
            .map_err(std::io::Error::from)
    })
    .await
    .map_err(|_| std::io::Error::other("no-clobber publication task failed"))?
}

#[cfg(unix)]
async fn sync_directory(path: PathBuf) -> std::io::Result<()> {
    let directory = tokio::fs::File::open(path).await?;
    directory.sync_all().await
}

#[cfg(unix)]
async fn sync_directory_chain_with<F, Fut>(
    plan: &DirectorySyncPlan,
    destination: &Path,
    mut sync: F,
) -> ZaiResult<()>
where
    F: FnMut(PathBuf) -> Fut,
    Fut: std::future::Future<Output = std::io::Result<()>>,
{
    for directory in &plan.directories {
        sync(directory.clone())
            .await
            .map_err(|_| directory_sync_failed(destination))?;
    }
    Ok(())
}

fn target_exists(dest: &Path) -> ZaiError {
    ZaiError::FileError {
        code: codes::SDK_IO,
        message: format!("download target already exists: {}", dest.display()),
    }
}

#[cfg(any(unix, test))]
fn directory_sync_failed(dest: &Path) -> ZaiError {
    ZaiError::FileError {
        code: codes::SDK_IO,
        message: format!(
            "download was published at {} but directory-chain durability could not be confirmed",
            dest.display()
        ),
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

    async fn closed_partial(dir: &Path, body: &[u8]) -> PartialFile {
        use tokio::io::AsyncWriteExt;

        let mut partial = PartialFile::new(dir).await.unwrap();
        partial.write_all(body).await.unwrap();
        let mut file = partial.file.take().unwrap();
        file.flush().await.unwrap();
        file.sync_all().await.unwrap();
        drop(file);
        partial
    }

    fn cleanup_path(partial: &PartialFile) -> PathBuf {
        partial.cleanup_path.as_ref().unwrap().clone()
    }

    fn partial_files(dir: &Path) -> Vec<PathBuf> {
        std::fs::read_dir(dir)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| {
                path.file_name()
                    .is_some_and(|name| name.to_string_lossy().ends_with(".part"))
            })
            .collect()
    }

    fn publication_error(kind: std::io::ErrorKind) -> std::io::Result<()> {
        Err(std::io::Error::from(kind))
    }

    async fn wait_until_missing(path: &Path) {
        tokio::time::timeout(std::time::Duration::from_secs(2), async {
            while path.exists() {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("deferred partial-file cleanup did not finish");
    }

    async fn prepared_atomic_download(dest: &Path) -> ZaiResult<AtomicDownload> {
        let destination = std::path::absolute(dest).map_err(ZaiError::from)?;
        let directory_sync = DirectorySyncPlan::capture_before_create(&destination).await?;
        if let Some(parent) = destination.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(ZaiError::from)?;
        }
        AtomicDownload::new(&destination, directory_sync).await
    }

    async fn atomic_download(dest: &Path, body: bytes::Bytes) -> ZaiResult<usize> {
        let mut download = prepared_atomic_download(dest).await?;
        download.write_chunk(&body).await?;
        download.commit().await
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn directory_sync_plan_orders_zero_one_and_many_new_levels() {
        let root = tempfile::tempdir().unwrap();

        let zero = root.path().join("zero.bin");
        let zero_plan = DirectorySyncPlan::capture_before_create(&zero)
            .await
            .unwrap();
        assert_eq!(zero_plan.directories, [root.path().to_path_buf()]);

        let one_parent = root.path().join("one");
        let one = one_parent.join("out.bin");
        let one_plan = DirectorySyncPlan::capture_before_create(&one)
            .await
            .unwrap();
        assert_eq!(
            one_plan.directories,
            [one_parent, root.path().to_path_buf()]
        );

        let first = root.path().join("many");
        let second = first.join("a");
        let third = second.join("b");
        let many = third.join("out.bin");
        let many_plan = DirectorySyncPlan::capture_before_create(&many)
            .await
            .unwrap();
        assert_eq!(
            many_plan.directories,
            [third, second, first, root.path().to_path_buf()]
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn injected_directory_sync_runs_deepest_to_preexisting_anchor() {
        use std::sync::{Arc, Mutex};

        let root = tempfile::tempdir().unwrap();
        let parent = root.path().join("new").join("nested");
        let dest = parent.join("out.bin");
        let plan = DirectorySyncPlan::capture_before_create(&dest)
            .await
            .unwrap();
        let expected = plan.directories.clone();
        tokio::fs::create_dir_all(&parent).await.unwrap();
        let mut download = AtomicDownload::new(&dest, plan).await.unwrap();
        download.write_chunk(b"complete").await.unwrap();
        let seen = Arc::new(Mutex::new(Vec::new()));

        let written = download
            .commit_with_directory_sync({
                let seen = Arc::clone(&seen);
                move |directory| {
                    seen.lock().unwrap().push(directory);
                    std::future::ready(Ok(()))
                }
            })
            .await
            .unwrap();

        assert_eq!(written, b"complete".len());
        assert_eq!(*seen.lock().unwrap(), expected);
        assert_eq!(std::fs::read(&dest).unwrap(), b"complete");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn every_directory_sync_failure_keeps_the_published_target() {
        use std::sync::{Arc, Mutex};

        let root = tempfile::tempdir().unwrap();
        for fail_at in 0..4 {
            let parent = root
                .path()
                .join(format!("case-{fail_at}"))
                .join("nested")
                .join("deepest");
            let dest = parent.join("out.bin");
            let plan = DirectorySyncPlan::capture_before_create(&dest)
                .await
                .unwrap();
            let expected = plan.directories.clone();
            assert_eq!(expected.len(), 4);
            tokio::fs::create_dir_all(&parent).await.unwrap();
            let mut download = AtomicDownload::new(&dest, plan).await.unwrap();
            download.write_chunk(b"complete").await.unwrap();
            let seen = Arc::new(Mutex::new(Vec::new()));
            let calls = Arc::new(AtomicUsize::new(0));

            let error = download
                .commit_with_directory_sync({
                    let seen = Arc::clone(&seen);
                    let calls = Arc::clone(&calls);
                    move |directory| {
                        seen.lock().unwrap().push(directory);
                        let call = calls.fetch_add(1, Ordering::SeqCst);
                        std::future::ready(if call == fail_at {
                            Err(std::io::Error::other("injected directory sync failure"))
                        } else {
                            Ok(())
                        })
                    }
                })
                .await
                .expect_err("injected directory sync failure was ignored");

            assert_eq!(error.code(), Some(codes::SDK_IO));
            assert!(
                error
                    .message()
                    .contains("directory-chain durability could not be confirmed")
            );
            assert_eq!(&*seen.lock().unwrap(), &expected[..=fail_at]);
            assert_eq!(std::fs::read(&dest).unwrap(), b"complete");
            assert!(partial_files(&parent).is_empty());
        }
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn cancellation_during_directory_sync_keeps_the_published_target() {
        let root = tempfile::tempdir().unwrap();
        let parent = root.path().join("created").join("nested");
        let dest = parent.join("out.bin");
        let plan = DirectorySyncPlan::capture_before_create(&dest)
            .await
            .unwrap();
        tokio::fs::create_dir_all(&parent).await.unwrap();
        let mut download = AtomicDownload::new(&dest, plan).await.unwrap();
        download.write_chunk(b"complete").await.unwrap();
        let sync_started = std::sync::Arc::new(tokio::sync::Notify::new());

        let commit = tokio::spawn(download.commit_with_directory_sync({
            let sync_started = std::sync::Arc::clone(&sync_started);
            move |_directory| {
                let sync_started = std::sync::Arc::clone(&sync_started);
                async move {
                    sync_started.notify_one();
                    std::future::pending::<std::io::Result<()>>().await
                }
            }
        }));

        tokio::time::timeout(std::time::Duration::from_secs(2), sync_started.notified())
            .await
            .expect("directory sync did not start after publication");
        commit.abort();
        assert!(commit.await.unwrap_err().is_cancelled());

        assert_eq!(std::fs::read(&dest).unwrap(), b"complete");
        assert!(partial_files(&parent).is_empty());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn real_unix_directory_chain_sync_smoke() {
        let root = tempfile::tempdir().unwrap();
        let dest = root.path().join("created").join("nested").join("out.bin");

        let written = atomic_download(&dest, bytes::Bytes::from_static(b"durable"))
            .await
            .unwrap();

        assert_eq!(written, b"durable".len());
        assert_eq!(std::fs::read(&dest).unwrap(), b"durable");
        assert!(partial_files(dest.parent().unwrap()).is_empty());
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

    #[tokio::test]
    async fn unsupported_hard_link_uses_noclobber_fallback() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("out.bin");
        let mut partial = closed_partial(dir.path(), b"fallback body").await;
        let private_path = cleanup_path(&partial);

        partial
            .finish_publication(&dest, publication_error(std::io::ErrorKind::Unsupported))
            .await
            .unwrap();

        assert_eq!(std::fs::read(&dest).unwrap(), b"fallback body");
        assert!(partial.cleanup_path.is_none());
        assert!(!private_path.exists());
        assert!(partial_files(dir.path()).is_empty());
    }

    #[tokio::test]
    async fn unsupported_fallback_never_clobbers_an_existing_target() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("out.bin");
        std::fs::write(&dest, b"winner").unwrap();
        let mut partial = closed_partial(dir.path(), b"loser").await;
        let private_path = cleanup_path(&partial);

        let error = partial
            .finish_publication(&dest, publication_error(std::io::ErrorKind::Unsupported))
            .await
            .unwrap_err();

        assert_eq!(error.code(), Some(codes::SDK_IO));
        assert!(error.message().contains("target already exists"));
        assert_eq!(std::fs::read(&dest).unwrap(), b"winner");
        assert!(partial.cleanup_path.is_none());
        assert!(!private_path.exists());
        assert!(partial_files(dir.path()).is_empty());
    }

    #[tokio::test]
    async fn permission_denied_does_not_fallback_and_drop_cleans_partial() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("out.bin");
        let mut partial = closed_partial(dir.path(), b"private").await;
        let private_path = cleanup_path(&partial);

        let error = partial
            .finish_publication(
                &dest,
                publication_error(std::io::ErrorKind::PermissionDenied),
            )
            .await
            .unwrap_err();

        assert_eq!(error.code(), Some(codes::SDK_IO));
        assert!(!dest.exists(), "PermissionDenied must not invoke fallback");
        assert_eq!(
            partial.cleanup_path.as_deref(),
            Some(private_path.as_path())
        );
        assert!(private_path.exists());

        drop(partial);
        wait_until_missing(&private_path).await;
        assert!(partial_files(dir.path()).is_empty());
    }

    #[tokio::test]
    async fn concurrent_unsupported_fallbacks_have_exactly_one_winner() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("out.bin");
        let mut first = closed_partial(dir.path(), b"first fallback").await;
        let mut second = closed_partial(dir.path(), b"second fallback").await;
        let first_unsupported = publication_error(std::io::ErrorKind::Unsupported);
        let second_unsupported = publication_error(std::io::ErrorKind::Unsupported);

        let (first_result, second_result) = tokio::join!(
            first.finish_publication(&dest, first_unsupported),
            second.finish_publication(&dest, second_unsupported),
        );

        let loser = match (first_result, second_result) {
            (Err(loser), Ok(())) | (Ok(()), Err(loser)) => loser,
            (first, second) => {
                panic!("exactly one fallback publisher must win: {first:?}, {second:?}")
            },
        };
        assert_eq!(loser.code(), Some(codes::SDK_IO));
        assert!(loser.message().contains("target already exists"));
        let body = std::fs::read(&dest).unwrap();
        assert!(body == b"first fallback" || body == b"second fallback");
        assert!(first.cleanup_path.is_none());
        assert!(second.cleanup_path.is_none());
        assert!(partial_files(dir.path()).is_empty());
    }

    #[tokio::test]
    async fn single_component_destination_gets_an_absolute_nonempty_parent() {
        let relative = PathBuf::from(format!(
            ".zai-atomic-single-{}-{:016x}.bin",
            std::process::id(),
            fastrand::u64(..)
        ));
        let expected = std::path::absolute(&relative).unwrap();
        assert!(!expected.exists());

        let download = prepared_atomic_download(&relative).await.unwrap();
        let private_path = cleanup_path(&download.partial);

        assert_eq!(download.destination, expected);
        assert!(download.destination.is_absolute());
        assert_eq!(
            private_path.parent(),
            download.destination.parent(),
            "the private file and destination must share the captured parent"
        );
        assert!(
            download
                .destination
                .parent()
                .is_some_and(|parent| !parent.as_os_str().is_empty())
        );

        drop(download);
        wait_until_missing(&private_path).await;
        assert!(!expected.exists());
    }

    #[test]
    fn drop_without_a_runtime_uses_the_synchronous_cleanup_fallback() {
        let dir = tempfile::tempdir().unwrap();
        let partial = PartialFile::new_blocking(dir.path()).unwrap();
        let private_path = cleanup_path(&partial);
        assert!(private_path.exists());

        drop(partial);

        assert!(!private_path.exists());
        assert!(partial_files(dir.path()).is_empty());
    }

    #[test]
    fn deferred_cleanup_budget_is_bounded_and_released_by_raii() {
        static TEST_BUDGET: CleanupBudget = CleanupBudget::new(2);

        let first = TEST_BUDGET.try_acquire().unwrap();
        let second = TEST_BUDGET.try_acquire().unwrap();
        assert!(TEST_BUDGET.try_acquire().is_none());
        assert_eq!(TEST_BUDGET.active.load(Ordering::Acquire), 2);

        drop(first);
        let replacement = TEST_BUDGET.try_acquire().unwrap();
        assert_eq!(TEST_BUDGET.active.load(Ordering::Acquire), 2);

        drop(second);
        drop(replacement);
        assert_eq!(TEST_BUDGET.active.load(Ordering::Acquire), 0);
    }

    #[test]
    fn queued_cleanup_guard_survives_runtime_shutdown_timeout() {
        use std::sync::{Arc, Condvar, Mutex, mpsc};
        use std::time::Duration;

        let dir = tempfile::tempdir().unwrap();
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .max_blocking_threads(1)
            .build()
            .unwrap();
        let partial = runtime.block_on(PartialFile::new(dir.path())).unwrap();
        let private_path = cleanup_path(&partial);

        // Occupy the only blocking worker so PartialFile::drop can only queue
        // its guarded cleanup job.
        let gate = Arc::new((Mutex::new(false), Condvar::new()));
        let worker_gate = gate.clone();
        let (started_tx, started_rx) = mpsc::sync_channel(1);
        let (done_tx, done_rx) = mpsc::sync_channel(1);
        drop(runtime.spawn_blocking(move || {
            started_tx.send(()).unwrap();
            let (lock, wake) = &*worker_gate;
            let mut released = lock.lock().unwrap();
            while !*released {
                released = wake.wait(released).unwrap();
            }
            done_tx.send(()).unwrap();
        }));
        started_rx.recv_timeout(Duration::from_secs(2)).unwrap();

        runtime.block_on(async move { drop(partial) });
        assert!(
            private_path.exists(),
            "cleanup unexpectedly ran past the gate"
        );

        // A bounded shutdown may return while queued blocking work remains.
        // The armed cleanup job must retain sole ownership until the blocking
        // pool can either run or discard it.
        runtime.shutdown_timeout(Duration::from_millis(50));
        assert!(
            private_path.exists(),
            "shutdown_timeout unexpectedly ran work past the blocking gate"
        );

        // Release the blocking worker and verify the queued guard is eventually
        // finalized, whether Tokio runs the job or discards its closure.
        let (lock, wake) = &*gate;
        *lock.lock().unwrap() = true;
        wake.notify_all();
        done_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        while private_path.exists() && std::time::Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(1));
        }
        assert!(!private_path.exists(), "queued cleanup ownership was lost");
    }

    #[test]
    fn directory_sync_failure_reports_published_destination_without_rollback() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("published.bin");
        std::fs::write(&dest, b"complete").unwrap();

        let error = directory_sync_failed(&dest);

        assert_eq!(error.code(), Some(codes::SDK_IO));
        assert!(matches!(error, ZaiError::FileError { .. }));
        assert!(error.message().contains("download was published at"));
        assert!(error.message().contains(&dest.display().to_string()));
        assert!(
            error
                .message()
                .contains("directory-chain durability could not be confirmed")
        );
        assert_eq!(std::fs::read(&dest).unwrap(), b"complete");
    }
}
