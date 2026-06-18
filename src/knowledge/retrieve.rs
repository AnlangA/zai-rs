use std::sync::Arc;

use super::types::KnowledgeDetailResponse;
use crate::{
    ZaiResult,
    client::{
        endpoints::{ApiBase, EndpointConfig, join_url, paths},
        http::{HttpClient, HttpClientConfig, parse_typed_response},
    },
};

/// Knowledge detail request (GET /llm-application/open/knowledge/{id})
pub struct KnowledgeRetrieveRequest {
    /// Bearer API key
    pub key: String,
    url: String,
    endpoint_config: EndpointConfig,
    api_base: ApiBase,
    id: String,
    http_config: Arc<HttpClientConfig>,
    _body: (),
}

impl KnowledgeRetrieveRequest {
    /// Build a retrieve request with id
    pub fn new(key: String, id: impl AsRef<str>) -> Self {
        let id = id.as_ref().to_string();
        let endpoint_config = EndpointConfig::default();
        let api_base = ApiBase::LlmApplication;
        let url = endpoint_config.url(&api_base, &join_url(paths::KNOWLEDGE, &id));
        Self {
            key,
            url,
            endpoint_config,
            api_base,
            id,
            http_config: Arc::new(HttpClientConfig::default()),
            _body: (),
        }
    }

    fn rebuild_url(&mut self) {
        self.url = self
            .endpoint_config
            .url(&self.api_base, &join_url(paths::KNOWLEDGE, &self.id));
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
    pub async fn send(&self) -> ZaiResult<KnowledgeDetailResponse> {
        let resp = self.get().await?;
        let parsed = parse_typed_response::<KnowledgeDetailResponse>(resp).await?;
        Ok(parsed)
    }
}

impl HttpClient for KnowledgeRetrieveRequest {
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

/// Alias for symmetry with other modules
pub type KnowledgeRetrieveResponse = KnowledgeDetailResponse;
