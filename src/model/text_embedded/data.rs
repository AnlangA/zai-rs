use std::sync::Arc;

use super::{
    request::{EmbeddingBody, EmbeddingDimensions, EmbeddingInput, EmbeddingModel},
    response::EmbeddingResponse,
};
use crate::client::{
    endpoints::{ApiBase, EndpointConfig, paths},
    http::{HttpClient, HttpClientConfig, parse_typed_response},
};

/// Text Embedding request client (JSON POST)
///
/// Builder for the embeddings endpoint. Construct with
/// [`EmbeddingRequest::new`], tune with the `with_*` methods, then call
/// [`EmbeddingRequest::send`].
pub struct EmbeddingRequest {
    /// Zhipu AI API key used for `Authorization: Bearer …`.
    pub key: String,
    url: String,
    endpoint_config: EndpointConfig,
    api_base: ApiBase,
    http_config: Arc<HttpClientConfig>,
    body: EmbeddingBody,
}

impl EmbeddingRequest {
    /// Create a new embedding request for the given model and input.
    pub fn new(key: impl Into<String>, model: EmbeddingModel, input: EmbeddingInput) -> Self {
        let endpoint_config = EndpointConfig::default();
        let api_base = ApiBase::PaasV4;
        let url = endpoint_config.url(&api_base, paths::EMBEDDINGS);
        let body = EmbeddingBody::new(model, input);
        Self {
            key: key.into(),
            url,
            endpoint_config,
            api_base,
            http_config: Arc::new(HttpClientConfig::default()),
            body,
        }
    }

    fn rebuild_url(&mut self) {
        self.url = self.endpoint_config.url(&self.api_base, paths::EMBEDDINGS);
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

    /// Set the embedding vector dimensionality.
    pub fn with_dimensions(mut self, dims: EmbeddingDimensions) -> Self {
        self.body = self.body.with_dimensions(dims);
        self
    }

    /// Optional: validate constraints before sending
    pub fn validate(&self) -> crate::ZaiResult<()> {
        self.body.validate_model_constraints().map_err(|e| {
            crate::client::error::ZaiError::ApiError {
                code: crate::client::error::codes::SDK_VALIDATION,
                message: format!("Validation error: {:?}", e),
            }
        })
    }

    /// Send the request and parse typed response.
    /// Automatically runs `validate()` before sending.
    pub async fn send(&self) -> crate::ZaiResult<EmbeddingResponse> {
        if let Err(e) = self.validate() {
            return Err(crate::client::error::ZaiError::ApiError {
                code: crate::client::error::codes::SDK_VALIDATION,
                message: format!("validation failed: {}", e),
            });
        }
        let resp: reqwest::Response = self.post().await?;
        let parsed = parse_typed_response::<EmbeddingResponse>(resp).await?;
        Ok(parsed)
    }

    #[deprecated(note = "Use send() instead")]
    /// Deprecated: use `send()`.
    pub async fn execute(&self) -> crate::ZaiResult<EmbeddingResponse> {
        self.send().await
    }
}

impl HttpClient for EmbeddingRequest {
    type Body = EmbeddingBody;
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
