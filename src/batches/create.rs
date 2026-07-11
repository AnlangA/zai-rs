use serde::{Deserialize, Serialize};
use serde_json::Value;
use validator::Validate;

use crate::{
    ZaiResult,
    client::{ApiFamily, ZaiClient},
};

/// Endpoint for batch requests
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BatchEndpoint {
    /// Chat completions endpoint
    #[serde(rename = "/v4/chat/completions")]
    ChatCompletions,
}

/// Request body for creating a batch task
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CreateBatchBody {
    /// ID of the uploaded .jsonl file (purpose must be "batch")
    #[validate(length(min = 1))]
    pub input_file_id: String,

    /// Endpoint to be used for all requests in the batch
    pub endpoint: BatchEndpoint,

    /// Whether to auto delete input file after processing (default: true)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_delete_input_file: Option<bool>,

    /// Arbitrary metadata for task management and tracking (up to 16 kv pairs)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
}

impl CreateBatchBody {
    /// Create a new batch body from an input file id and target endpoint
    /// (auto-delete defaults to `true`).
    pub fn new(input_file_id: impl Into<String>, endpoint: BatchEndpoint) -> Self {
        Self {
            input_file_id: input_file_id.into(),
            endpoint,
            auto_delete_input_file: Some(true),
            metadata: None,
        }
    }

    /// Set auto delete flag
    pub fn with_auto_delete_input_file(mut self, v: bool) -> Self {
        self.auto_delete_input_file = Some(v);
        self
    }

    /// Set metadata object
    pub fn with_metadata(mut self, v: Value) -> Self {
        self.metadata = Some(v);
        self
    }
}

/// Create batch request (POST /paas/v4/batches)
pub struct CreateBatchRequest {
    /// Request body.
    pub body: CreateBatchBody,
}

impl CreateBatchRequest {
    /// Build a new create-batch request with required fields
    pub fn new(input_file_id: impl Into<String>, endpoint: BatchEndpoint) -> Self {
        let body = CreateBatchBody::new(input_file_id, endpoint);
        Self { body }
    }

    /// Set auto-delete flag (default true)
    pub fn with_auto_delete_input_file(mut self, v: bool) -> Self {
        self.body = self.body.with_auto_delete_input_file(v);
        self
    }

    /// Set metadata object
    pub fn with_metadata(mut self, v: serde_json::Value) -> Self {
        self.body = self.body.with_metadata(v);
        self
    }

    /// Validate body using `validator`
    pub fn validate(&self) -> ZaiResult<()> {
        self.body.validate().map_err(std::convert::Into::into)
    }

    /// Send request via a [`ZaiClient`] and parse typed response
    pub async fn send_via(&self, client: &ZaiClient) -> ZaiResult<CreateBatchResponse> {
        self.validate()?;

        let url = client
            .endpoints()
            .resolve(ApiFamily::PaasV4, &["batches"])?;
        client
            .send_json::<_, CreateBatchResponse>("POST", url, &self.body)
            .await
    }
}

/// Response type for creating a batch task (same as a single item)
pub type CreateBatchResponse = super::types::BatchItem;
