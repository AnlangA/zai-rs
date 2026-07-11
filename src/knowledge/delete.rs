use std::sync::Arc;

use crate::client::http::{HttpClientConfig, parse_typed_response, send_empty_request};
use crate::client::v2::ZaiClient;

/// Knowledge delete request (DELETE /llm-application/open/knowledge/{id})
///
/// Credentials and transport live on the [`ZaiClient`], passed to
/// [`send_via`](Self::send_via).
pub struct KnowledgeDeleteRequest {
    id: String,
}

impl KnowledgeDeleteRequest {
    /// Build a delete request with target id.
    pub fn new(id: impl Into<String>) -> Self {
        Self { id: id.into() }
    }

    /// Send via a [`ZaiClient`] and parse the typed response.
    pub async fn send_via(&self, client: &ZaiClient) -> crate::ZaiResult<KnowledgeDeleteResponse> {
        let url = client.endpoints().resolve(
            crate::client::v2::ApiFamily::LlmApplication,
            &["knowledge", &self.id],
        )?;
        let config = transport_config_from_client(client);
        let resp = send_empty_request(
            reqwest::Method::DELETE,
            url,
            client.secret().expose(),
            Arc::new(config),
        )
        .await?;
        parse_typed_response::<KnowledgeDeleteResponse>(resp).await
    }
}

/// Delete response envelope without data
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, validator::Validate)]
pub struct KnowledgeDeleteResponse {
    /// Business status code.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<i64>,
    /// Human-readable message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// Server timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<u64>,
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
