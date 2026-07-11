use std::sync::Arc;

use super::types::BatchItem;
use crate::{
    ZaiResult,
    client::{
        http::{HttpClientConfig, parse_typed_response, send_empty_request},
        {ApiFamily, ZaiClient},
    },
};

/// Retrieve a batch task by ID (GET /paas/v4/batches/{batch_id})
pub struct BatchesRetrieveRequest {
    batch_id: String,
}

impl BatchesRetrieveRequest {
    /// Create a new retrieve request with required path parameter `batch_id`.
    pub fn new(batch_id: impl Into<String>) -> Self {
        Self {
            batch_id: batch_id.into(),
        }
    }

    /// Send request via a [`ZaiClient`] and parse typed response as a single
    /// BatchItem
    pub async fn send_via(&self, client: &ZaiClient) -> ZaiResult<BatchesRetrieveResponse> {
        let url = client
            .endpoints()
            .resolve(ApiFamily::PaasV4, &["batches", &self.batch_id])?;
        let config = transport_config_from_client(client);
        let resp: reqwest::Response = send_empty_request(
            reqwest::Method::GET,
            url,
            client.secret().expose(),
            Arc::new(config),
        )
        .await?;
        let parsed = parse_typed_response::<BatchesRetrieveResponse>(resp).await?;
        Ok(parsed)
    }
}

/// Response type: a single Batch object
pub type BatchesRetrieveResponse = BatchItem;

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
