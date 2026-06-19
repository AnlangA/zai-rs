use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use validator::Validate;

use crate::{
    ZaiResult,
    client::{
        endpoints::{ApiBase, EndpointConfig, paths},
        http::{HttpClient, HttpClientConfig, parse_typed_response},
    },
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
    /// Zhipu AI API key used for `Authorization: Bearer …`.
    pub key: String,
    url: String,
    endpoint_config: EndpointConfig,
    api_base: ApiBase,
    http_config: Arc<HttpClientConfig>,
    /// Request body.
    pub body: CreateBatchBody,
}

impl CreateBatchRequest {
    /// Build a new create-batch request with required fields
    pub fn new(key: String, input_file_id: impl Into<String>, endpoint: BatchEndpoint) -> Self {
        let body = CreateBatchBody::new(input_file_id, endpoint);
        let endpoint_config = EndpointConfig::default();
        let api_base = ApiBase::PaasV4;
        let url = endpoint_config.url(&api_base, paths::BATCHES);
        Self {
            key,
            url,
            endpoint_config,
            api_base,
            http_config: Arc::new(HttpClientConfig::default()),
            body,
        }
    }

    fn rebuild_url(&mut self) {
        self.url = self.endpoint_config.url(&self.api_base, paths::BATCHES);
    }

    /// Override the base URL (uses [`ApiBase::Custom`]).
    pub fn with_base_url(mut self, base: impl Into<String>) -> Self {
        self.api_base = ApiBase::Custom(base.into());
        self.rebuild_url();
        self
    }

    /// Replace the full [`EndpointConfig`] used to resolve URLs.
    pub fn with_endpoint_config(mut self, endpoint_config: EndpointConfig) -> Self {
        self.endpoint_config = endpoint_config;
        self.rebuild_url();
        self
    }

    /// Replace the HTTP client configuration (timeouts, retries, …).
    pub fn with_http_config(mut self, config: HttpClientConfig) -> Self {
        self.http_config = Arc::new(config);
        self
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
        self.body.validate().map_err(|e| e.into())
    }

    /// Send request and parse typed response
    pub async fn send(&self) -> ZaiResult<CreateBatchResponse> {
        self.validate()?;

        let resp: reqwest::Response = self.post().await?;

        let parsed = parse_typed_response::<CreateBatchResponse>(resp).await?;
        Ok(parsed)
    }
}

impl HttpClient for CreateBatchRequest {
    type Body = CreateBatchBody;
    type ApiUrl = String;
    type ApiKey = String;

    /// Resolved target URL for the request.
    fn api_url(&self) -> &Self::ApiUrl {
        &self.url
    }
    /// API key used for `Authorization: Bearer …`.
    fn api_key(&self) -> &Self::ApiKey {
        &self.key
    }
    /// Serialized request body.
    fn body(&self) -> &Self::Body {
        &self.body
    }

    /// HTTP client configuration (timeouts, retries, …).
    fn http_config(&self) -> Arc<HttpClientConfig> {
        Arc::clone(&self.http_config)
    }
}

/// Response type for creating a batch task (same as a single item)
pub type CreateBatchResponse = super::types::BatchItem;
