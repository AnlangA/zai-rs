use super::types::BatchItem;
use crate::{ZaiResult, client::ZaiClient};

/// Cancel a running batch (POST /paas/v4/batches/{batch_id}/cancel)
pub struct BatchCancelRequest {
    batch_id: String,
}

impl BatchCancelRequest {
    /// Create a new cancel request for the given batch_id
    pub fn new(batch_id: impl Into<String>) -> Self {
        Self {
            batch_id: batch_id.into(),
        }
    }

    /// Send the request via a [`ZaiClient`] and parse typed response
    pub async fn send_via(&self, client: &ZaiClient) -> ZaiResult<BatchCancelResponse> {
        crate::client::validation::require_non_blank(&self.batch_id, "batch_id")?;
        let route = crate::client::routes::BATCHES_CANCEL;
        let url = client.endpoints().resolve_route(route, &[&self.batch_id])?;
        client
            .send_empty::<BatchCancelResponse>(route.method(), url)
            .await
    }
}

/// Response type: a single Batch object
pub type BatchCancelResponse = BatchItem;
