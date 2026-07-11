use std::sync::Arc;

use super::types::KnowledgeDetailResponse;
use crate::client::http::{HttpClientConfig, parse_typed_response, send_empty_request};
use crate::client::v2::ZaiClient;

/// Knowledge detail request (GET /llm-application/open/knowledge/{id})
///
/// Credentials and transport live on the [`ZaiClient`], passed to
/// [`send_via`](Self::send_via).
pub struct KnowledgeRetrieveRequest {
    id: String,
}

impl KnowledgeRetrieveRequest {
    /// Build a retrieve request with id.
    pub fn new(id: impl Into<String>) -> Self {
        Self { id: id.into() }
    }

    /// Send via a [`ZaiClient`] and parse the typed response.
    pub async fn send_via(&self, client: &ZaiClient) -> crate::ZaiResult<KnowledgeDetailResponse> {
        let url = client.endpoints().resolve(
            crate::client::v2::ApiFamily::LlmApplication,
            &["knowledge", &self.id],
        )?;
        let config = transport_config_from_client(client);
        let resp = send_empty_request(
            reqwest::Method::GET,
            url,
            client.secret().expose(),
            Arc::new(config),
        )
        .await?;
        parse_typed_response::<KnowledgeDetailResponse>(resp).await
    }
}

/// Alias for symmetry with other modules
pub type KnowledgeRetrieveResponse = KnowledgeDetailResponse;

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
