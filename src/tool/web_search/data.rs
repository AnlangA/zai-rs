use crate::{
    ZaiResult,
    client::ZaiClient,
    tool::web_search::{request::*, response::*},
};

/// Web search API request builder (P05: routes through [`ZaiClient`]).
pub struct WebSearchRequest {
    body: WebSearchBody,
}

impl WebSearchRequest {
    pub fn new(search_query: String, search_engine: SearchEngine) -> Self {
        Self {
            body: WebSearchBody::new(search_query, search_engine),
        }
    }

    pub fn with_body(body: WebSearchBody) -> Self {
        Self { body }
    }

    pub fn with_search_intent(mut self, enabled: bool) -> Self {
        self.body = self.body.with_search_intent(enabled);
        self
    }
    pub fn with_count(mut self, count: i32) -> Self {
        self.body = self.body.with_count(count);
        self
    }
    pub fn with_domain_filter(mut self, domain: impl Into<String>) -> Self {
        self.body = self.body.with_domain_filter(domain.into());
        self
    }
    pub fn with_recency_filter(mut self, filter: SearchRecencyFilter) -> Self {
        self.body = self.body.with_recency_filter(filter);
        self
    }
    pub fn with_content_size(mut self, size: ContentSize) -> Self {
        self.body = self.body.with_content_size(size);
        self
    }
    pub fn with_request_id(mut self, request_id: impl Into<String>) -> Self {
        self.body = self.body.with_request_id(request_id.into());
        self
    }
    pub fn with_user_id(mut self, user_id: impl Into<String>) -> Self {
        self.body = self.body.with_user_id(user_id.into());
        self
    }

    pub fn validate(&self) -> ZaiResult<()> {
        self.body.validate_constraints()
    }

    pub async fn send_via(&self, client: &ZaiClient) -> ZaiResult<WebSearchResponse> {
        self.validate()?;
        let route = crate::client::routes::TOOLS_WEB_SEARCH;
        let url = client.endpoints().resolve_route(route, &[])?;
        client
            .send_json::<_, WebSearchResponse>(route.method(), url, &self.body)
            .await
    }
}
