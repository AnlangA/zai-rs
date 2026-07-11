//! Images request types (plan P06) — async image generation.

use crate::ZaiResult;
use crate::client::{ApiFamily, ZaiClient};

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
        let url = client
            .endpoints()
            .resolve(ApiFamily::PaasV4, &["async", "images", "generations"])?;
        client
            .send_json::<_, AsyncImageGenerationResponse>("POST", url, &self.body)
            .await
    }
}

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------
