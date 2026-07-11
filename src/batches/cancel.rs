use serde::{Deserialize, Serialize};

use super::types::BatchItem;
use crate::{
    ZaiResult,
    client::{ApiFamily, ZaiClient},
};

/// Empty body for cancel API (serializes to `{}`)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CancelBatchBody {}

/// Cancel a running batch (POST /paas/v4/batches/{batch_id}/cancel)
pub struct CancelBatchRequest {
    batch_id: String,
    /// Empty JSON body
    body: CancelBatchBody,
}

impl CancelBatchRequest {
    /// Create a new cancel request for the given batch_id
    pub fn new(batch_id: impl Into<String>) -> Self {
        Self {
            batch_id: batch_id.into(),
            body: CancelBatchBody::default(),
        }
    }

    /// Send the request via a [`ZaiClient`] and parse typed response
    pub async fn send_via(&self, client: &ZaiClient) -> ZaiResult<CancelBatchResponse> {
        let url = client
            .endpoints()
            .resolve(ApiFamily::PaasV4, &["batches", &self.batch_id, "cancel"])?;
        client
            .send_json::<_, CancelBatchResponse>("POST", url, &self.body)
            .await
    }
}

/// Response type: a single Batch object
pub type CancelBatchResponse = BatchItem;
