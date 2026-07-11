use std::sync::Arc;

use crate::client::{
    http::{HttpClientConfig, parse_typed_response, send_empty_request},
    v2::{ApiFamily, ZaiClient},
};

/// File delete request (DELETE /paas/v4/files/{file_id})
pub struct FileDeleteRequest {
    file_id: String,
}

impl FileDeleteRequest {
    /// Create a new delete request for the given file id.
    pub fn new(file_id: impl Into<String>) -> Self {
        Self {
            file_id: file_id.into(),
        }
    }

    /// Send delete request via a [`ZaiClient`] and parse typed response.
    pub async fn send_via(
        &self,
        client: &ZaiClient,
    ) -> crate::ZaiResult<super::response::FileDeleteResponse> {
        let url = client
            .endpoints()
            .resolve(ApiFamily::PaasV4, &["files", &self.file_id])?;
        let config = transport_config_from_client(client);
        let resp = send_empty_request(
            reqwest::Method::DELETE,
            url,
            client.secret().expose(),
            Arc::new(config),
        )
        .await?;
        parse_typed_response::<super::response::FileDeleteResponse>(resp).await
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
