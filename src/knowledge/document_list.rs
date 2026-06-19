use std::sync::Arc;

use super::types::DocumentListResponse;
use crate::{
    ZaiResult,
    client::{
        endpoints::{ApiBase, EndpointConfig, build_query, paths},
        http::{HttpClient, HttpClientConfig, parse_typed_response},
    },
};

/// Query parameters for listing documents under a knowledge base
#[derive(Debug, Clone, serde::Serialize, validator::Validate)]
pub struct DocumentListQuery {
    /// Knowledge base id (required)
    #[validate(length(min = 1))]
    pub knowledge_id: String,
    /// Page index (default 1)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(range(min = 1))]
    pub page: Option<u32>,
    /// Page size (default 10)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(range(min = 1))]
    pub size: Option<u32>,
    /// Document name filter
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(length(min = 1))]
    pub word: Option<String>,
}

impl DocumentListQuery {
    /// Create a new query for the given knowledge base id (page 1, size 10).
    pub fn new(knowledge_id: impl Into<String>) -> Self {
        Self {
            knowledge_id: knowledge_id.into(),
            page: Some(1),
            size: Some(10),
            word: None,
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
    /// Filter by document name.
    pub fn with_word(mut self, word: impl Into<String>) -> Self {
        self.word = Some(word.into());
        self
    }
}

/// Document list request (GET /llm-application/open/document)
pub struct DocumentListRequest {
    /// Bearer API key
    pub key: String,
    url: String,
    endpoint_config: EndpointConfig,
    api_base: ApiBase,
    http_config: Arc<HttpClientConfig>,
    query: Option<DocumentListQuery>,
    _body: (),
}

impl DocumentListRequest {
    /// Create a new document-list request (no query set yet).
    pub fn new(key: String) -> Self {
        let endpoint_config = EndpointConfig::default();
        let api_base = ApiBase::LlmApplication;
        let url = endpoint_config.url(&api_base, paths::DOCUMENT);
        Self {
            key,
            url,
            endpoint_config,
            api_base,
            http_config: Arc::new(HttpClientConfig::default()),
            query: None,
            _body: (),
        }
    }

    fn rebuild_url(&mut self) {
        let endpoint = self.endpoint_config.url(&self.api_base, paths::DOCUMENT);
        let mut params: Vec<(&str, String)> = Vec::new();
        if let Some(q) = &self.query {
            params.push(("knowledge_id", q.knowledge_id.clone()));
            if let Some(page) = q.page.as_ref() {
                params.push(("page", page.to_string()));
            }
            if let Some(size) = q.size.as_ref() {
                params.push(("size", size.to_string()));
            }
            if let Some(word) = q.word.as_ref() {
                params.push(("word", word.clone()));
            }
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
    pub fn with_query(mut self, q: DocumentListQuery) -> Self {
        self.query = Some(q);
        self.rebuild_url();
        self
    }

    /// Validate query, rebuild URL, then send
    pub async fn send_with_query(
        mut self,
        q: &DocumentListQuery,
    ) -> ZaiResult<DocumentListResponse> {
        use validator::Validate;
        q.validate()?;
        self.query = Some(q.clone());
        self.rebuild_url();
        self.send().await
    }

    /// Send and parse typed response
    pub async fn send(&self) -> ZaiResult<DocumentListResponse> {
        let resp = self.get().await?;
        let parsed = parse_typed_response::<DocumentListResponse>(resp).await?;
        Ok(parsed)
    }
}

impl HttpClient for DocumentListRequest {
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
