use super::types::KnowledgeListResponse;
use crate::ZaiResult;
use crate::client::ZaiClient;

/// Query parameters for knowledge list API
#[derive(Debug, Clone, serde::Serialize, validator::Validate)]
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

    /// Build the `(&str, String)` query pairs (in stable order) used to form
    /// the request URL.
    fn pairs(&self) -> Vec<(&'static str, String)> {
        let mut params: Vec<(&'static str, String)> = Vec::new();
        if let Some(page) = self.page.as_ref() {
            params.push(("page", page.to_string()));
        }
        if let Some(size) = self.size.as_ref() {
            params.push(("size", size.to_string()));
        }
        params
    }
}

impl Default for KnowledgeListQuery {
    fn default() -> Self {
        Self::new()
    }
}

/// Knowledge list request (GET /llm-application/open/knowledge)
///
/// Credentials and transport live on the [`ZaiClient`], passed to
/// [`send_via`](Self::send_via).
pub struct KnowledgeListRequest {
    query: KnowledgeListQuery,
}

impl KnowledgeListRequest {
    /// Create a new knowledge-list request (default query: page 1, size 10).
    pub fn new() -> Self {
        Self {
            query: KnowledgeListQuery::new(),
        }
    }

    /// Apply a query (replaces the current one).
    pub fn with_query(mut self, q: KnowledgeListQuery) -> Self {
        self.query = q;
        self
    }

    /// Send via a [`ZaiClient`] and parse the typed response.
    pub async fn send_via(&self, client: &ZaiClient) -> ZaiResult<KnowledgeListResponse> {
        use validator::Validate;

        self.query.validate()?;
        let params = self.query.pairs();
        let route = crate::client::routes::KNOWLEDGE_LIST;
        client
            .operation(route)
            .with_query(params)
            .send_empty::<KnowledgeListResponse>()
            .await
    }
}

impl Default for KnowledgeListRequest {
    fn default() -> Self {
        Self::new()
    }
}
