use std::sync::Arc;

use super::types::KnowledgeCapacityResponse;
use crate::client::ZaiClient;
use crate::client::http::{HttpClientConfig, parse_typed_response, send_empty_request};

/// Knowledge capacity request (GET /llm-application/open/knowledge/capacity)
///
/// Credentials and transport live on the [`ZaiClient`], passed to
/// [`send_via`](Self::send_via).
#[allow(clippy::new_without_default)]
#[derive(Default)]
pub struct KnowledgeCapacityRequest {
    _body: (),
}

impl KnowledgeCapacityRequest {
    /// Build a capacity request.
    pub fn new() -> Self {
        Self { _body: () }
    }

    /// Send via a [`ZaiClient`] and parse the typed response.
    pub async fn send_via(
        &self,
        client: &ZaiClient,
    ) -> crate::ZaiResult<KnowledgeCapacityResponse> {
        let url = client.endpoints().resolve(
            crate::client::ApiFamily::LlmApplication,
            &["knowledge", "capacity"],
        )?;
        let config = transport_config_from_client(client);
        let resp = send_empty_request(
            reqwest::Method::GET,
            url,
            client.secret().expose(),
            Arc::new(config),
        )
        .await?;
        parse_typed_response::<KnowledgeCapacityResponse>(resp).await
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
