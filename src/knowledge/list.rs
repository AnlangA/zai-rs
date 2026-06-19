use std::sync::Arc;

use super::types::KnowledgeListResponse;
use crate::{
    ZaiResult,
    client::{
        endpoints::{ApiBase, EndpointConfig, build_query, paths},
        http::{HttpClient, HttpClientConfig, parse_typed_response},
    },
};

/// Query parameters for knowledge list API
#[derive(Debug, Clone, Default, serde::Serialize, validator::Validate)]
pub struct KnowledgeListQuery {
    /// Page index starting from 1 (default 1)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(range(min = 1))]
    pub page: Option<u32>,
    /// Page size (default 10)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(range(min = 1))]
    pub size: Option<u32>,
}

impl KnowledgeListQuery {
    /// Create a new query (page 1, size 10).
    pub fn new() -> Self {
        Self {
            page: Some(1),
            size: Some(10),
        }
    }
    /// Set the page index (1-based).
    pub fn with_page(mut self, page: u32) -> Self {
        self.page = Some(page);
        self
    }
    /// Set the page size.
    pub fn with_size(mut self, size: u32) -> Self {
        self.size = Some(size);
        self
    }
}

/// Knowledge list request (GET /llm-application/open/knowledge)
pub struct KnowledgeListRequest {
    /// Bearer API key
    pub key: String,
    url: String,
    endpoint_config: EndpointConfig,
    api_base: ApiBase,
    http_config: Arc<HttpClientConfig>,
    query: KnowledgeListQuery,
    _body: (),
}

impl KnowledgeListRequest {
    /// Create a new knowledge-list request (default query: page 1, size 10).
    pub fn new(key: String) -> Self {
        let endpoint_config = EndpointConfig::default();
        let api_base = ApiBase::LlmApplication;
        let url = endpoint_config.url(&api_base, paths::KNOWLEDGE);
        Self {
            key,
            url,
            endpoint_config,
            api_base,
            http_config: Arc::new(HttpClientConfig::default()),
            query: KnowledgeListQuery::new(),
            _body: (),
        }
    }

    fn rebuild_url(&mut self) {
        let endpoint = self.endpoint_config.url(&self.api_base, paths::KNOWLEDGE);
        let mut params: Vec<(&str, String)> = Vec::new();
        if let Some(page) = self.query.page.as_ref() {
            params.push(("page", page.to_string()));
        }
        if let Some(size) = self.query.size.as_ref() {
            params.push(("size", size.to_string()));
        }
        self.url = build_query(&endpoint, params);
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

    /// Apply query by rebuilding internal URL
    pub fn with_query(mut self, q: KnowledgeListQuery) -> Self {
        self.query = q;
        self.rebuild_url();
        self
    }

    /// Send request and parse typed response
    pub async fn send(&self) -> ZaiResult<KnowledgeListResponse> {
        let resp = self.get().await?;
        let parsed = parse_typed_response::<KnowledgeListResponse>(resp).await?;
        Ok(parsed)
    }

    /// Validate query, rebuild URL then send
    pub async fn send_with_query(
        mut self,
        q: &KnowledgeListQuery,
    ) -> ZaiResult<KnowledgeListResponse> {
        use validator::Validate;
        q.validate()?;
        self.query = q.clone();
        self.rebuild_url();
        self.send().await
    }
}

impl HttpClient for KnowledgeListRequest {
    type Body = ();
    type ApiUrl = String;
    type ApiKey = String;

    /// Resolved target URL (with query string) for the request.
    fn api_url(&self) -> &Self::ApiUrl {
        &self.url
    }
    /// API key used for `Authorization: Bearer …`.
    fn api_key(&self) -> &Self::ApiKey {
        &self.key
    }
    /// Empty body placeholder (GET request).
    fn body(&self) -> &Self::Body {
        &self._body
    }

    /// HTTP client configuration (timeouts, retries, …).
    fn http_config(&self) -> Arc<HttpClientConfig> {
        Arc::clone(&self.http_config)
    }
}
