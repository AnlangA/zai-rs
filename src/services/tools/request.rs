//! Tools request types (plan P06) — layout parsing & reader.

use std::sync::Arc;

use crate::ZaiResult;
use crate::client::http::{HttpClientConfig, parse_typed_response, send_json_request};
use crate::client::v2::{ApiFamily, ZaiClient};

use super::response::{LayoutParsingResponse, ReaderResponse};

// ---------------------------------------------------------------------------
// LayoutParsingRequest
// ---------------------------------------------------------------------------

/// POST /layout_parsing — document layout analysis.
pub struct LayoutParsingRequest {
    pub body: serde_json::Value,
}

impl LayoutParsingRequest {
    pub fn new(body: serde_json::Value) -> Self {
        Self { body }
    }

    /// Send the request and parse a typed response.
    pub async fn send_via(&self, client: &ZaiClient) -> ZaiResult<LayoutParsingResponse> {
        let url = client
            .endpoints()
            .resolve(ApiFamily::PaasV4, &["layout_parsing"])?;
        let config = transport_config(client);
        let resp = send_json_request(
            reqwest::Method::POST,
            url,
            client.secret().expose(),
            &self.body,
            Arc::new(config),
        )
        .await?;
        parse_typed_response::<LayoutParsingResponse>(resp).await
    }
}

// ---------------------------------------------------------------------------
// ReaderRequest
// ---------------------------------------------------------------------------

/// POST /reader — document reading / content extraction.
pub struct ReaderRequest {
    pub body: serde_json::Value,
}

impl ReaderRequest {
    pub fn new(body: serde_json::Value) -> Self {
        Self { body }
    }

    /// Send the request and parse a typed response.
    pub async fn send_via(&self, client: &ZaiClient) -> ZaiResult<ReaderResponse> {
        let url = client.endpoints().resolve(ApiFamily::PaasV4, &["reader"])?;
        let config = transport_config(client);
        let resp = send_json_request(
            reqwest::Method::POST,
            url,
            client.secret().expose(),
            &self.body,
            Arc::new(config),
        )
        .await?;
        parse_typed_response::<ReaderResponse>(resp).await
    }
}

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

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
