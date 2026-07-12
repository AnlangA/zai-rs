//! Open-format synchronous file-parsing request.

use crate::ZaiResult;
use crate::client::ZaiClient;

/// Response for a synchronous file parsing request.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct FileParseSyncResponse {
    /// Parsed content (open schema).
    #[serde(default)]
    pub data: serde_json::Value,
}

/// Synchronous file-parsing request for `POST /files/parser/sync`.
///
/// The request body is intentionally an open JSON value and is not validated by
/// the SDK. Callers must supply the object shape expected by the service.
pub struct FileParseSyncRequest {
    /// JSON body passed to the endpoint unchanged.
    pub body: serde_json::Value,
}

impl FileParseSyncRequest {
    /// Create a request from an open-format JSON body.
    pub fn new(body: serde_json::Value) -> Self {
        Self { body }
    }

    /// Send the request and parse a typed response.
    pub async fn send_via(&self, client: &ZaiClient) -> ZaiResult<FileParseSyncResponse> {
        let route = crate::client::routes::FILES_PARSE_SYNC;
        let url = client.endpoints().resolve_route(route, &[])?;
        client
            .send_json::<_, FileParseSyncResponse>(route.method(), url, &self.body)
            .await
    }
}
