use std::sync::Arc;

use super::{
    request::{RerankBody, RerankModel},
    response::RerankResponse,
};
use crate::{
    ZaiResult,
    client::{
        endpoints::{ApiBase, EndpointConfig, paths},
        http::{HttpClient, HttpClientConfig, parse_typed_response},
    },
};

/// Text Rerank request client (JSON POST)
///
/// Builder for the rerank endpoint. Construct with [`RerankRequest::new`],
/// tune with the `with_*` methods, then call [`RerankRequest::send`].
pub struct RerankRequest {
    /// Zhipu AI API key used for `Authorization: Bearer …`.
    pub key: String,
    url: String,
    endpoint_config: EndpointConfig,
    api_base: ApiBase,
    http_config: Arc<HttpClientConfig>,
    body: RerankBody,
}

impl RerankRequest {
    /// Create a new rerank request for a query and a set of candidate
    /// documents.
    pub fn new(key: impl Into<String>, query: impl Into<String>, documents: Vec<String>) -> Self {
        let endpoint_config = EndpointConfig::default();
        let api_base = ApiBase::PaasV4;
        let url = endpoint_config.url(&api_base, paths::RERANK);
        let body = RerankBody::new(RerankModel::Rerank, query, documents);
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
        self.url = self.endpoint_config.url(&self.api_base, paths::RERANK);
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

    /// Set how many top-ranked documents to return.
    pub fn with_top_n(mut self, n: usize) -> Self {
        self.body = self.body.with_top_n(n);
        self
    }
    /// Whether to include the document text in the response.
    pub fn with_return_documents(mut self, v: bool) -> Self {
        self.body = self.body.with_return_documents(v);
        self
    }
    /// Whether to include raw relevance scores in the response.
    pub fn with_return_raw_scores(mut self, v: bool) -> Self {
        self.body = self.body.with_return_raw_scores(v);
        self
    }
    /// Set the client-side request id.
    pub fn with_request_id(mut self, v: impl Into<String>) -> Self {
        self.body = self.body.with_request_id(v);
        self
    }
    /// Set the end-user id.
    pub fn with_user_id(mut self, v: impl Into<String>) -> Self {
        self.body = self.body.with_user_id(v);
        self
    }

    /// Optional: validate constraints before sending
    pub fn validate(&self) -> ZaiResult<()> {
        self.body
            .validate_constraints()
            .map_err(|e| crate::client::error::ZaiError::ApiError {
                code: crate::client::error::codes::SDK_VALIDATION,
                message: format!("Validation error: {:?}", e),
            })
    }

    /// Send the request and parse typed response.
    /// Automatically runs `validate()` before sending.
    pub async fn send(&self) -> ZaiResult<RerankResponse> {
        self.validate()?;
        let resp: reqwest::Response = self.post().await?;
        let parsed = parse_typed_response::<RerankResponse>(resp).await?;
        Ok(parsed)
    }

    #[deprecated(note = "Use send() instead")]
    /// Deprecated: use `send()`.
    pub async fn execute(&self) -> ZaiResult<RerankResponse> {
        self.send().await
    }
}

impl HttpClient for RerankRequest {
    type Body = RerankBody;
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
