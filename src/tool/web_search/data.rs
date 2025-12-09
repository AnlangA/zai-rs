use crate::ZaiResult;
use crate::client::http::HttpClient;
use crate::tool::web_search::{request::*, response::*};
use async_trait::async_trait;

/// Web search API client
pub struct WebSearchRequest {
    /// API key for authentication
    pub key: String,
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
        Self {
            key,
            body: WebSearchBody::new(search_query, search_engine),
        }
    }

    /// Create a web search request with a pre-configured body
    pub fn with_body(key: String, body: WebSearchBody) -> Self {
        Self { key, body }
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
        let parsed = resp.json::<WebSearchResponse>().await?;
        Ok(parsed)
    }
}

#[async_trait]
impl HttpClient for WebSearchRequest {
    type Body = WebSearchBody;
    type ApiUrl = &'static str;
    type ApiKey = String;

    fn api_url(&self) -> &Self::ApiUrl {
        &"https://open.bigmodel.cn/api/paas/v4/web_search"
    }

    fn api_key(&self) -> &Self::ApiKey {
        &self.key
    }

    fn body(&self) -> &Self::Body {
        &self.body
    }
}
