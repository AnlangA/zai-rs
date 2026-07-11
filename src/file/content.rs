use std::sync::Arc;

use crate::client::{
    http::{HttpClientConfig, send_empty_request},
    {ApiFamily, ZaiClient},
};

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
        let url = client
            .endpoints()
            .resolve(ApiFamily::PaasV4, &["files", &self.file_id, "content"])?;
        let config = transport_config_from_client(client);
        // `send_empty_request` routes through the retry pipeline, which returns
        // `Ok` only for a 2xx response (any non-2xx is converted to `Err`
        // there), so the response here is guaranteed successful — no status
        // re-check needed.
        let resp: reqwest::Response = send_empty_request(
            reqwest::Method::GET,
            url,
            client.secret().expose(),
            Arc::new(config),
        )
        .await?;
        let bytes = resp.bytes().await?;
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

fn transport_config_from_client(client: &ZaiClient) -> HttpClientConfig {
    let t = client.transport();
    HttpClientConfig {
        timeout: std::time::Duration::from_secs(t.request_timeout.as_secs()),
        max_retries: u32::from(t.max_attempts).saturating_sub(1),
        enable_compression: t.enable_compression,
        retry_delay: crate::client::http::RetryDelay::Exponential {
            base: std::time::Duration::from_millis(500),
            max: std::time::Duration::from_secs(5),
        },
        enable_logging: false,
        mask_sensitive_data: true,
    }
}
