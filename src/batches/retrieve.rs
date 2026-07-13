use super::types::BatchItem;
use crate::{ZaiResult, client::ZaiClient};

/// Retrieve a batch task by ID (GET /paas/v4/batches/{batch_id})
pub struct BatchGetRequest {
    batch_id: String,
}

impl BatchGetRequest {
    /// Create a new retrieve request with required path parameter `batch_id`.
    pub fn new(batch_id: impl Into<String>) -> Self {
        Self {
            batch_id: batch_id.into(),
        }
    }

    /// Send request via a [`ZaiClient`] and parse typed response as a single
    /// BatchItem
    pub async fn send_via(&self, client: &ZaiClient) -> ZaiResult<BatchGetResponse> {
        crate::client::validation::require_non_blank(&self.batch_id, "batch_id")?;
        let route = crate::client::routes::BATCHES_GET;
        client
            .operation(route)
            .with_parameters([self.batch_id.as_str()])
            .send_empty::<BatchGetResponse>()
            .await
    }
}

/// Response type: a single Batch object
pub type BatchGetResponse = BatchItem;
