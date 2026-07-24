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
    /// Parent directories are created when missing. Chunks are written directly
    /// to a private same-directory temporary file, then flushed, synced and
    /// published with an atomic no-clobber hard link. The destination filesystem
    /// must support hard links. The file contents are synced before publication,
    /// but the parent directory is not currently synced, so success does not
    /// promise directory-entry survival across sudden power loss.
    /// Transfer failure or future cancellation removes the `.part` file.
    /// Returns the number of bytes written.
    pub async fn send_to_via<P: AsRef<std::path::Path>>(
        &self,
        client: &ZaiClient,
        path: P,
    ) -> crate::ZaiResult<usize> {
        crate::client::validation::require_non_blank(&self.file_id, "file_id")?;

        let p = path.as_ref();

        // Use `tokio::fs` so a slow/networked filesystem or a large file can't
        // stall the async runtime worker (mirrors the upload-path fix).
        if let Some(parent) = p.parent()
            && !parent.as_os_str().is_empty()
        {
            tokio::fs::create_dir_all(parent).await?;
        }
        // Reserve the private file before network I/O so an existing target
        // fails fast and cancellation during the handshake is still cleaned.
        let mut download = crate::client::transport::download::AtomicDownload::new(p).await?;
        let mut stream = FileContentStream {
            inner: self.fetch_stream_via(client).await?,
        };
        while let Some(chunk) = stream.next().await {
            download.write_chunk(&chunk?).await?;
        }
        download.commit().await
    }
}
