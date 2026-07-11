use std::sync::Arc;

use super::types::DocumentImageListResponse;
use crate::client::http::{HttpClientConfig, parse_typed_response, send_empty_request};
use crate::client::v2::ZaiClient;

/// Retrieve parsed image index-url mapping for a document (POST, no body)
///
/// Credentials and transport live on the [`ZaiClient`], passed to
/// [`send_via`](Self::send_via).
pub struct DocumentImageListRequest {
    document_id: String,
}

impl DocumentImageListRequest {
    /// Create a new request with the target document id
    pub fn new(document_id: impl Into<String>) -> Self {
        Self {
            document_id: document_id.into(),
        }
    }

    /// Send the POST request via a [`ZaiClient`] and parse the typed response.
    pub async fn send_via(
        &self,
        client: &ZaiClient,
    ) -> crate::ZaiResult<DocumentImageListResponse> {
        let url = client.endpoints().resolve(
            crate::client::v2::ApiFamily::LlmApplication,
            &["document", "slice", "image_list", &self.document_id],
        )?;
        let config = transport_config_from_client(client);
        let resp = send_empty_request(
            reqwest::Method::POST,
            url,
            client.secret().expose(),
            Arc::new(config),
        )
        .await?;
        parse_typed_response::<DocumentImageListResponse>(resp).await
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
