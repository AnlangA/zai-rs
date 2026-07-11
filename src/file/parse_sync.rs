//! File parse sync request (POST /paas/v4/files/parser/sync) — plan P06.

use crate::ZaiResult;
use crate::client::ZaiClient;

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
        let route = crate::client::routes::FILES_PARSE_SYNC;
        let url = client.endpoints().resolve_route(route, &[])?;
        client
            .send_json::<_, FileParseSyncResponse>(route.method(), url, &self.body)
            .await
    }
}
