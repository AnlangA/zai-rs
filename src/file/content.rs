use crate::client::ZaiClient;

/// Buffered bytes returned by the file-content operation.
pub type ByteStream = Vec<u8>;

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

    /// Send the request via a [`ZaiClient`] and return raw bytes of the file
    /// content.
    pub async fn send_via(&self, client: &ZaiClient) -> crate::ZaiResult<ByteStream> {
        Ok(self.fetch_bytes_via(client).await?.to_vec())
    }

    async fn fetch_bytes_via(&self, client: &ZaiClient) -> crate::ZaiResult<bytes::Bytes> {
        crate::client::validation::require_non_blank(&self.file_id, "file_id")?;
        let route = crate::client::routes::FILES_GET_CONTENT;
        let url = client.endpoints().resolve_route(route, &[&self.file_id])?;
        client.send_empty_bytes(route.method(), url).await
    }

    /// Send via a [`ZaiClient`] and write the file content to `path`.
    ///
    /// Parent directories are created when missing. The response is buffered,
    /// written to a private same-directory temporary file, synced, and then
    /// published atomically. An existing destination is never replaced.
    /// Returns the number of bytes written.
    pub async fn send_to_via<P: AsRef<std::path::Path>>(
        &self,
        client: &ZaiClient,
        path: P,
    ) -> crate::ZaiResult<usize> {
        let bytes = self.fetch_bytes_via(client).await?;

        let p = path.as_ref();

        // Use `tokio::fs` so a slow/networked filesystem or a large file can't
        // stall the async runtime worker (mirrors the upload-path fix).
        if let Some(parent) = p.parent()
            && !parent.as_os_str().is_empty()
        {
            tokio::fs::create_dir_all(parent).await?;
        }
        let length = bytes.len();
        crate::client::transport::download::atomic_download(p, bytes).await?;
        Ok(length)
    }
}
