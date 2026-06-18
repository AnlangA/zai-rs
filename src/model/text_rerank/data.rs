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
pub struct RerankRequest {
    pub key: String,
    url: String,
    endpoint_config: EndpointConfig,
    api_base: ApiBase,
    http_config: Arc<HttpClientConfig>,
    body: RerankBody,
}

impl RerankRequest {
    pub fn new(key: String, query: impl Into<String>, documents: Vec<String>) -> Self {
        let endpoint_config = EndpointConfig::default();
        let api_base = ApiBase::PaasV4;
        let url = endpoint_config.url(&api_base, paths::RERANK);
        let body = RerankBody::new(RerankModel::Rerank, query, documents);
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
        self.url = self.endpoint_config.url(&self.api_base, paths::RERANK);
    }

    pub fn with_base_url(mut self, base: impl Into<String>) -> Self {
        self.api_base = ApiBase::Custom(base.into());
        self.rebuild_url();
        self
    }

    pub fn with_endpoint_config(mut self, endpoint_config: EndpointConfig) -> Self {
        self.endpoint_config = endpoint_config;
        self.rebuild_url();
        self
    }

    pub fn with_http_config(mut self, config: HttpClientConfig) -> Self {
        self.http_config = Arc::new(config);
        self
    }

    pub fn with_top_n(mut self, n: usize) -> Self {
        self.body = self.body.with_top_n(n);
        self
    }
    pub fn with_return_documents(mut self, v: bool) -> Self {
        self.body = self.body.with_return_documents(v);
        self
    }
    pub fn with_return_raw_scores(mut self, v: bool) -> Self {
        self.body = self.body.with_return_raw_scores(v);
        self
    }
    pub fn with_request_id(mut self, v: impl Into<String>) -> Self {
        self.body = self.body.with_request_id(v);
        self
    }
    pub fn with_user_id(mut self, v: impl Into<String>) -> Self {
        self.body = self.body.with_user_id(v);
        self
    }

    /// Optional: validate constraints before sending
    pub fn validate(&self) -> ZaiResult<()> {
        self.body
            .validate_constraints()
            .map_err(|e| crate::client::error::ZaiError::ApiError {
                code: 1200,
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

    fn api_url(&self) -> &Self::ApiUrl {
        &self.url
    }
    fn api_key(&self) -> &Self::ApiKey {
        &self.key
    }
    fn body(&self) -> &Self::Body {
        &self.body
    }
    fn http_config(&self) -> Arc<HttpClientConfig> {
        Arc::clone(&self.http_config)
    }
}
