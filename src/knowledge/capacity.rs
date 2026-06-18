use std::sync::Arc;

use super::types::KnowledgeCapacityResponse;
use crate::client::{
    endpoints::{ApiBase, EndpointConfig, paths},
    http::{HttpClient, HttpClientConfig, parse_typed_response},
};

/// Knowledge capacity request (GET /llm-application/open/knowledge/capacity)
pub struct KnowledgeCapacityRequest {
    /// Bearer API key
    pub key: String,
    url: String,
    endpoint_config: EndpointConfig,
    api_base: ApiBase,
    http_config: Arc<HttpClientConfig>,
    _body: (),
}

impl KnowledgeCapacityRequest {
    /// Build a capacity request
    pub fn new(key: String) -> Self {
        let endpoint_config = EndpointConfig::default();
        let api_base = ApiBase::LlmApplication;
        let url = endpoint_config.url(&api_base, paths::KNOWLEDGE_CAPACITY);
        Self {
            key,
            url,
            endpoint_config,
            api_base,
            http_config: Arc::new(HttpClientConfig::default()),
            _body: (),
        }
    }

    fn rebuild_url(&mut self) {
        self.url = self
            .endpoint_config
            .url(&self.api_base, paths::KNOWLEDGE_CAPACITY);
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

    /// Send and parse typed response
    pub async fn send(&self) -> crate::ZaiResult<KnowledgeCapacityResponse> {
        let resp = self.get().await?;

        let parsed = parse_typed_response::<KnowledgeCapacityResponse>(resp).await?;

        Ok(parsed)
    }
}

impl HttpClient for KnowledgeCapacityRequest {
    type Body = ();
    type ApiUrl = String;
    type ApiKey = String;

    fn api_url(&self) -> &Self::ApiUrl {
        &self.url
    }
    fn api_key(&self) -> &Self::ApiKey {
        &self.key
    }
    fn body(&self) -> &Self::Body {
        &self._body
    }

    fn http_config(&self) -> Arc<HttpClientConfig> {
        Arc::clone(&self.http_config)
    }
}
