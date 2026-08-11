use std::pin::Pin;
use std::task::{Context, Poll};

use futures_util::{Stream, StreamExt};

use crate::client::ZaiClient;

/// Buffered bytes returned by the file-content operation.
pub type ByteStream = Vec<u8>;

/// Pull-based response stream returned by [`FileContentRequest::stream_via`].
///
/// Each item is one [`bytes::Bytes`] chunk. The transport retains at most the
/// chunks held by the caller/HTTP stack, so a slow consumer naturally applies
/// backpressure instead of growing an SDK-side file buffer.
pub struct FileContentStream {
    inner: crate::client::transport::FileByteStream,
}

impl FileContentStream {
    /// Await the next file chunk, or `None` after a complete response.
    ///
    /// A transfer error can arrive after earlier chunks; after yielding that
    /// error the stream terminates.
    pub async fn next(&mut self) -> Option<crate::ZaiResult<bytes::Bytes>> {
        self.inner.next().await
    }
}

impl Stream for FileContentStream {
    type Item = crate::ZaiResult<bytes::Bytes>;

    fn poll_next(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.inner.as_mut().poll_next(context)
    }
}

/// File content request (GET /paas/v4/files/{file_id}/content)
pub struct FileContentRequest {
    file_id: String,
}

impl FileContentRequest {
    /// Create a new content request for the given file id.
    pub fn new(file_id: impl Into<String>) -> Self {
        Self {
            file_id: file_id.into(),
        }
    }

    /// Send via a [`ZaiClient`] and collect the complete file in memory.
    ///
    /// This convenience API preserves the original `Vec<u8>` return type. Use
    /// [`stream_via`](Self::stream_via) or [`send_to_via`](Self::send_to_via)
    /// to keep memory bounded for large files.
    pub async fn send_via(&self, client: &ZaiClient) -> crate::ZaiResult<ByteStream> {
        let mut stream = self.stream_via(client).await?;
        let mut content = Vec::new();
        while let Some(chunk) = stream.next().await {
            content.extend_from_slice(&chunk?);
        }
        Ok(content)
    }

    /// Send via a [`ZaiClient`] and yield bounded, pull-based file chunks.
    ///
    /// GET authentication, same-origin redirects, retry policy, MIME/business
    /// errors and transport deadlines remain enforced by the client. The total
    /// decoded body is capped at 128 MiB. Transient failures may be retried only
    /// before the first chunk is returned; once any chunk is visible, an
    /// interruption is yielded as an error without replaying the request.
    /// Attempt and overall deadlines are absolute and therefore continue to run
    /// while the consumer is paused between polls.
    pub async fn stream_via(&self, client: &ZaiClient) -> crate::ZaiResult<FileContentStream> {
        crate::client::validation::require_non_blank(&self.file_id, "file_id")?;
        Ok(FileContentStream {
            inner: self.fetch_stream_via(client).await?,
        })
    }

    async fn fetch_stream_via(
        &self,
        client: &ZaiClient,
    ) -> crate::ZaiResult<crate::client::transport::FileByteStream> {
        let route = crate::client::routes::FILES_GET_CONTENT;
        client
            .operation(route)
            .with_parameters([self.file_id.as_str()])
            .send_empty_file_stream()
            .await
    }

    /// Send via a [`ZaiClient`] and write the file content to `path`.
    ///
    /// A relative path is anchored to the process working directory once, before
    /// any parent directories are created. This is a lexical absolute-path
    /// conversion, not symlink canonicalization; a later working-directory change
    /// therefore cannot split directory creation, the private partial, and final
    /// publication across different locations.
    ///
    /// Parent directories are created when missing. Chunks are written directly
    /// to a private same-directory temporary file (mode `0600` on Unix), then
    /// flushed, file-synced, closed, and published without replacing an existing
    /// destination. Publication prefers a hard link. If the filesystem explicitly
    /// reports hard links as unsupported, a platform-aware no-clobber persistence
    /// fallback is used; it preserves the no-overwrite guarantee but may leave the
    /// private temporary name behind if its final cleanup fails. Other hard-link
    /// errors are returned rather than silently weakening the contract.
    ///
    /// On Unix, successful publication is followed by bottom-up directory syncs:
    /// the destination's immediate parent, every newly created ancestor, and the
    /// first parent directory that existed before this call. Success therefore
    /// means the completed file, its destination entry, and any directory entries
    /// created for this download were synced. This lexical protocol does not
    /// canonicalize or pin symlinks and cannot protect against another process
    /// replacing path components while the download runs. Stable Rust has no
    /// portable directory-sync contract on Windows and other non-Unix targets;
    /// there the file is synced before publication, but directory-entry crash
    /// durability is not promised.
    ///
    /// A Unix directory-chain sync can fail after publication. In that case this
    /// method returns an error even though the complete destination already exists
    /// and is not rolled back; callers should inspect/reconcile that path instead
    /// of blindly retrying the same destination. Cancellation can race with an
    /// already-dispatched publication operation, so callers should also reconcile
    /// the destination after cancellation; if present, it is complete and was
    /// never produced by overwriting an existing path.
    ///
    /// Transfer failure or cancellation makes a best-effort attempt to remove the
    /// private `.part` file. Drop closes the SDK file handle first. When a Tokio
    /// runtime is available and one of eight process-wide cleanup slots can be
    /// acquired, removal is deferred to the blocking pool; the bound covers both
    /// queued and running cleanup jobs. If no runtime is active, the budget is
    /// saturated, or the guarded cleanup cannot be armed, removal is attempted
    /// synchronously instead of creating an unbounded queue. Cleanup errors are not
    /// returned from Drop, and the SDK does not scan for stale partials at startup;
    /// a failed cleanup or abrupt process termination may therefore leave a private
    /// `.part` file for application/operator reconciliation.
    /// Returns the number of bytes written.
    pub async fn send_to_via<P: AsRef<std::path::Path>>(
        &self,
        client: &ZaiClient,
        path: P,
    ) -> crate::ZaiResult<usize> {
        crate::client::validation::require_non_blank(&self.file_id, "file_id")?;

        let requested_path = path.as_ref();
        // Keep the established validation result: Path::absolute("") would
        // otherwise reinterpret an empty target as the current directory.
        if requested_path.as_os_str().is_empty() {
            return Err(crate::ZaiError::ApiError {
                code: crate::client::error::codes::SDK_CONFIG,
                message: "download target must not be empty".to_string(),
            });
        }
        // Anchor relative destinations before create_dir_all. AtomicDownload
        // resolves again defensively, but doing it here prevents a concurrent
        // process-wide CWD change from splitting directory creation and final
        // publication across different roots.
        let p = std::path::absolute(requested_path).map_err(crate::ZaiError::from)?;

        // Use `tokio::fs` so a slow/networked filesystem or a large file can't
        // stall the async runtime worker (mirrors the upload-path fix).
        let directory_sync =
            crate::client::transport::download::DirectorySyncPlan::capture_before_create(&p)
                .await?;
        if let Some(parent) = p.parent()
            && !parent.as_os_str().is_empty()
        {
            tokio::fs::create_dir_all(parent).await?;
        }
        // Reserve the private file before network I/O so an existing target
        // fails fast and cancellation during the handshake is still cleaned.
        let mut download =
            crate::client::transport::download::AtomicDownload::new(&p, directory_sync).await?;
        let mut stream = FileContentStream {
            inner: self.fetch_stream_via(client).await?,
        };
        while let Some(chunk) = stream.next().await {
            download.write_chunk(&chunk?).await?;
        }
        download.commit().await
    }
}
