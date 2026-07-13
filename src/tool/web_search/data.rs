use crate::{
    ZaiResult,
    client::ZaiClient,
    tool::web_search::{request::*, response::*},
};

/// Builder for a standalone web-search request sent through a [`ZaiClient`].
pub struct WebSearchRequest {
    body: WebSearchBody,
}

impl WebSearchRequest {
    /// Create a request from a query and search engine.
    pub fn new(search_query: impl Into<String>, search_engine: SearchEngine) -> Self {
        Self {
            body: WebSearchBody::new(search_query, search_engine),
        }
    }

    /// Create a request from a fully configured body.
    pub fn with_body(body: WebSearchBody) -> Self {
        Self { body }
    }

    /// Enable or disable search-intent recognition.
    pub fn with_search_intent(mut self, enabled: bool) -> Self {
        self.body = self.body.with_search_intent(enabled);
        self
    }
    /// Set the number of results to return.
    ///
    /// The general range is 1–50. `SearchProSogou` accepts only multiples of
    /// ten in that range; [`validate`](Self::validate) enforces both rules.
    pub fn with_count(mut self, count: i32) -> Self {
        self.body = self.body.with_count(count);
        self
    }
    /// Restrict results to the supplied domain whitelist.
    pub fn with_domain_filter(mut self, domain: impl Into<String>) -> Self {
        self.body = self.body.with_domain_filter(domain);
        self
    }
    /// Restrict results by publication recency.
    pub fn with_recency_filter(mut self, filter: SearchRecencyFilter) -> Self {
        self.body = self.body.with_recency_filter(filter);
        self
    }
    /// Select how much source-page content each result should include.
    pub fn with_content_size(mut self, size: ContentSize) -> Self {
        self.body = self.body.with_content_size(size);
        self
    }
    /// Set the client-provided request identifier.
    pub fn with_request_id(mut self, request_id: impl Into<String>) -> Self {
        self.body = self.body.with_request_id(request_id);
        self
    }
    /// Set the end-user identifier used for abuse monitoring.
    pub fn with_user_id(mut self, user_id: impl Into<String>) -> Self {
        self.body = self.body.with_user_id(user_id);
        self
    }

    /// Validate query length, result count and identifier constraints.
    pub fn validate(&self) -> ZaiResult<()> {
        self.body.validate_constraints()
    }

    /// Validate, send and decode the web-search response.
    pub async fn send_via(&self, client: &ZaiClient) -> ZaiResult<WebSearchResponse> {
        self.validate()?;
        let route = crate::client::routes::TOOLS_WEB_SEARCH;
        client
            .operation(route)
            .send_json::<_, WebSearchResponse>(&self.body)
            .await
    }
}
