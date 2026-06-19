use std::sync::Arc;

use async_trait::async_trait;

use crate::{
    ZaiResult,
    client::{
        endpoints::{ApiBase, EndpointConfig, paths},
        http::{HttpClient, HttpClientConfig, parse_typed_response},
    },
    tool::web_search::{request::*, response::*},
};

/// Web search API client
pub struct WebSearchRequest {
    /// API key for authentication
    pub key: String,
    url: String,
    endpoint_config: EndpointConfig,
    api_base: ApiBase,
    http_config: Arc<HttpClientConfig>,
    /// Request body
    body: WebSearchBody,
}

impl WebSearchRequest {
    /// Create a new web search request
    ///
    /// # Arguments
    /// * `key` - API key for authentication
    /// * `search_query` - Search query content (max 70 characters)
    /// * `search_engine` - Search engine to use
    pub fn new(key: String, search_query: String, search_engine: SearchEngine) -> Self {
        let body = WebSearchBody::new(search_query, search_engine);
        Self::with_body(key, body)
    }

    /// Create a web search request with a pre-configured body
    pub fn with_body(key: String, body: WebSearchBody) -> Self {
        let endpoint_config = EndpointConfig::default();
        let api_base = ApiBase::PaasV4;
        let url = endpoint_config.url(&api_base, paths::WEB_SEARCH);
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
        self.url = self.endpoint_config.url(&self.api_base, paths::WEB_SEARCH);
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

    /// Enable search intent recognition
    pub fn with_search_intent(mut self, enabled: bool) -> Self {
        self.body = self.body.with_search_intent(enabled);
        self
    }

    /// Set the number of results to return
    pub fn with_count(mut self, count: i32) -> Self {
        self.body = self.body.with_count(count);
        self
    }

    /// Set domain filter for search results
    pub fn with_domain_filter(mut self, domain: String) -> Self {
        self.body = self.body.with_domain_filter(domain);
        self
    }

    /// Set time range filter for search results
    pub fn with_recency_filter(mut self, filter: SearchRecencyFilter) -> Self {
        self.body = self.body.with_recency_filter(filter);
        self
    }

    /// Set content size preference
    pub fn with_content_size(mut self, size: ContentSize) -> Self {
        self.body = self.body.with_content_size(size);
        self
    }

    /// Set custom request ID
    pub fn with_request_id(mut self, request_id: String) -> Self {
        self.body = self.body.with_request_id(request_id);
        self
    }

    /// Set user ID
    pub fn with_user_id(mut self, user_id: String) -> Self {
        self.body = self.body.with_user_id(user_id);
        self
    }

    /// Validate the request
    pub fn validate(&self) -> ZaiResult<()> {
        self.body.validate_constraints()
    }

    /// Send the web search request and return the response
    pub async fn send(&self) -> ZaiResult<WebSearchResponse> {
        self.validate()?;
        let resp: reqwest::Response = self.post().await?;
        let parsed = parse_typed_response::<WebSearchResponse>(resp).await?;
        Ok(parsed)
    }
}

#[async_trait]
impl HttpClient for WebSearchRequest {
    type Body = WebSearchBody;
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
