//! Request types for document layout parsing and content extraction.

use crate::ZaiResult;
use crate::client::ZaiClient;

use super::response::{LayoutParsingResponse, ReaderResponse};

// ---------------------------------------------------------------------------
// LayoutParsingRequest
// ---------------------------------------------------------------------------

/// POST /layout_parsing — document layout analysis.
pub struct LayoutParsingRequest {
    /// Open-schema request body sent as JSON.
    pub body: serde_json::Value,
}

impl LayoutParsingRequest {
    /// Create a document layout-parsing request.
    pub fn new(body: serde_json::Value) -> Self {
        Self { body }
    }

    /// Send the request and parse a typed response.
    pub async fn send_via(&self, client: &ZaiClient) -> ZaiResult<LayoutParsingResponse> {
        let route = crate::client::routes::TOOLS_LAYOUT;
        let url = client.endpoints().resolve_route(route, &[])?;
        client
            .send_json::<_, LayoutParsingResponse>(route.method(), url, &self.body)
            .await
    }
}

// ---------------------------------------------------------------------------
// ReaderRequest
// ---------------------------------------------------------------------------

/// POST /reader — document reading / content extraction.
pub struct ReaderRequest {
    /// Open-schema request body sent as JSON.
    pub body: serde_json::Value,
}

impl ReaderRequest {
    /// Create a document reader request.
    pub fn new(body: serde_json::Value) -> Self {
        Self { body }
    }

    /// Send the request and parse a typed response.
    pub async fn send_via(&self, client: &ZaiClient) -> ZaiResult<ReaderResponse> {
        let route = crate::client::routes::TOOLS_READER;
        let url = client.endpoints().resolve_route(route, &[])?;
        client
            .send_json::<_, ReaderResponse>(route.method(), url, &self.body)
            .await
    }
}
