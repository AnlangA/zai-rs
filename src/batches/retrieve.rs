use super::types::BatchItem;
use crate::{ZaiResult, client::ZaiClient};

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
        let route = crate::client::routes::BATCHES_GET;
        let url = client.endpoints().resolve_route(route, &[&self.batch_id])?;
        client
            .send_empty::<BatchesRetrieveResponse>(route.method(), url)
            .await
    }
}

/// Response type: a single Batch object
pub type BatchesRetrieveResponse = BatchItem;
