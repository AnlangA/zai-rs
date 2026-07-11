use std::sync::Arc;

use serde::{Deserialize, Serialize};

use super::types::BatchItem;
use crate::{
    ZaiResult,
    client::{
        http::{HttpClientConfig, parse_typed_response, send_json_request},
        v2::{ApiFamily, ZaiClient},
    },
};

/// Empty body for cancel API (serializes to `{}`)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CancelBatchBody {}

/// Cancel a running batch (POST /paas/v4/batches/{batch_id}/cancel)
pub struct CancelBatchRequest {
    batch_id: String,
    /// Empty JSON body
    body: CancelBatchBody,
}

impl CancelBatchRequest {
    /// Create a new cancel request for the given batch_id
    pub fn new(batch_id: impl Into<String>) -> Self {
        Self {
            batch_id: batch_id.into(),
            body: CancelBatchBody::default(),
        }
    }

    /// Send the request via a [`ZaiClient`] and parse typed response
    pub async fn send_via(&self, client: &ZaiClient) -> ZaiResult<CancelBatchResponse> {
        let url = client
            .endpoints()
            .resolve(ApiFamily::PaasV4, &["batches", &self.batch_id, "cancel"])?;
        let config = transport_config_from_client(client);
        let resp: reqwest::Response = send_json_request(
            reqwest::Method::POST,
            url,
            client.secret().expose(),
            &self.body,
            Arc::new(config),
        )
        .await?;
        let parsed = parse_typed_response::<CancelBatchResponse>(resp).await?;
        Ok(parsed)
    }
}

/// Response type: a single Batch object
pub type CancelBatchResponse = BatchItem;

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
