//! Images request types (plan P06) — async image generation.

use crate::ZaiResult;
use crate::client::ZaiClient;

use super::response::AsyncImageGenerationResponse;

// ---------------------------------------------------------------------------
// AsyncImageGenerationRequest
// ---------------------------------------------------------------------------

/// POST /async/images/generations — async image generation.
pub struct AsyncImageGenerationRequest {
    pub body: serde_json::Value,
}

impl AsyncImageGenerationRequest {
    pub fn new(body: serde_json::Value) -> Self {
        Self { body }
    }

    /// Send the request and parse a typed response.
    pub async fn send_via(&self, client: &ZaiClient) -> ZaiResult<AsyncImageGenerationResponse> {
        let route = crate::client::routes::IMAGES_GENERATE_ASYNC;
        let url = client.endpoints().resolve_route(route, &[])?;
        client
            .send_json::<_, AsyncImageGenerationResponse>(route.method(), url, &self.body)
            .await
    }
}

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------
