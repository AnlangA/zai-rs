use serde::{Deserialize, Serialize};
use serde_json::Value;
use validator::Validate;

use crate::client::http::HttpClient;

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
    pub fn new(input_file_id: impl Into<String>, endpoint: BatchEndpoint) -> Self {
        Self { input_file_id: input_file_id.into(), endpoint, auto_delete_input_file: Some(true), metadata: None }
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
    pub key: String,
    pub body: CreateBatchBody,
}

impl CreateBatchRequest {
    /// Build a new create-batch request with required fields
    pub fn new(key: String, input_file_id: impl Into<String>, endpoint: BatchEndpoint) -> Self {
        let body = CreateBatchBody::new(input_file_id, endpoint);
        Self { key, body }
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
    pub fn validate(&self) -> anyhow::Result<()> {
        self.body.validate().map_err(|e| anyhow::anyhow!(e))
    }

    /// Send request and parse typed response
    pub async fn send(&self) -> anyhow::Result<CreateBatchResponse> {
        self.validate()?;
        let resp: reqwest::Response = self.post().await?;
        let parsed = resp.json::<CreateBatchResponse>().await?;
        Ok(parsed)
    }
}

impl HttpClient for CreateBatchRequest {
    type Body = CreateBatchBody;
    type ApiUrl = &'static str;
    type ApiKey = String;

    fn api_url(&self) -> &Self::ApiUrl { &"https://open.bigmodel.cn/api/paas/v4/batches" }
    fn api_key(&self) -> &Self::ApiKey { &self.key }
    fn body(&self) -> &Self::Body { &self.body }
}

/// Response type for creating a batch task (same as a single item)
pub type CreateBatchResponse = super::types::BatchItem;

