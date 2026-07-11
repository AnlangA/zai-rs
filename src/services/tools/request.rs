//! Tools request types (plan P06) — layout parsing & reader.

use crate::ZaiResult;
use crate::client::{ApiFamily, ZaiClient};

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
        client
            .send_json::<_, LayoutParsingResponse>("POST", url, &self.body)
            .await
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
        client
            .send_json::<_, ReaderResponse>("POST", url, &self.body)
            .await
    }
}

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------
