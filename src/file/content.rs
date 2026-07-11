use crate::client::ZaiClient;

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
    pub async fn send_via(&self, client: &ZaiClient) -> crate::ZaiResult<Vec<u8>> {
        let route = crate::client::routes::FILES_GET_CONTENT;
        let url = client.endpoints().resolve_route(route, &[&self.file_id])?;
        let bytes = client.send_empty_bytes(route.method(), url).await?;
        Ok(bytes.to_vec())
    }

    /// Send via a [`ZaiClient`] and write the file content to `path`. It will
    /// create parent directories if missing. Returns the number of bytes
    /// written.
    pub async fn send_to_via<P: AsRef<std::path::Path>>(
        &self,
        client: &ZaiClient,
        path: P,
    ) -> crate::ZaiResult<usize> {
        let bytes = self.send_via(client).await?;

        let p = path.as_ref();

        // Use `tokio::fs` so a slow/networked filesystem or a large file can't
        // stall the async runtime worker (mirrors the upload-path fix).
        if let Some(parent) = p.parent()
            && !parent.as_os_str().is_empty()
        {
            tokio::fs::create_dir_all(parent).await?;
        }
        let mut f = tokio::fs::File::create(p).await?;
        tokio::io::AsyncWriteExt::write_all(&mut f, &bytes).await?;
        Ok(bytes.len())
    }
}
