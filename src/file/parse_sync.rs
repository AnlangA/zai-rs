//! File parse sync request (POST /paas/v4/files/parser/sync) — plan P06.

use std::sync::Arc;

use crate::ZaiResult;
use crate::client::http::{HttpClientConfig, parse_typed_response, send_json_request};
use crate::client::v2::{ApiFamily, ZaiClient};

/// Response for a synchronous file parsing request.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct FileParseSyncResponse {
    /// Parsed content (open schema).
    #[serde(default)]
    pub data: serde_json::Value,
}

/// POST /files/parser/sync — synchronous file parsing.
pub struct FileParseSyncRequest {
    pub body: serde_json::Value,
}

impl FileParseSyncRequest {
    pub fn new(body: serde_json::Value) -> Self {
        Self { body }
    }

    /// Send the request and parse a typed response.
    pub async fn send_via(&self, client: &ZaiClient) -> ZaiResult<FileParseSyncResponse> {
        let url = client
            .endpoints()
            .resolve(ApiFamily::PaasV4, &["files", "parser", "sync"])?;
        let config = transport_config(client);
        let resp = send_json_request(
            reqwest::Method::POST,
            url,
            client.secret().expose(),
            &self.body,
            Arc::new(config),
        )
        .await?;
        parse_typed_response::<FileParseSyncResponse>(resp).await
    }
}

fn transport_config(client: &ZaiClient) -> HttpClientConfig {
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
