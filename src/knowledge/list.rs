use super::types::KnowledgeListResponse;
use crate::ZaiResult;
use crate::client::ZaiClient;
use crate::pagination::PagePagination;

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

    /// Replace the page and size with validated pagination values.
    pub fn try_with_pagination(mut self, pagination: PagePagination) -> ZaiResult<Self> {
        let (page, page_size) = pagination.into_parts();
        self.page = Some(page);
        self.size = Some(page_size);
        Ok(self)
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

    /// Replace the request's page and size with validated pagination.
    pub fn try_with_pagination(mut self, pagination: PagePagination) -> ZaiResult<Self> {
        self.query = self.query.try_with_pagination(pagination)?;
        Ok(self)
    }

    /// Send via a [`ZaiClient`] and parse the typed response.
    pub async fn send_via(&self, client: &ZaiClient) -> ZaiResult<KnowledgeListResponse> {
        use validator::Validate;

        self.query.validate()?;
        let route = crate::client::routes::KNOWLEDGE_LIST;
        client
            .operation(route)
            .with_query(&self.query)?
            .send_empty::<KnowledgeListResponse>()
            .await
    }
}

impl Default for KnowledgeListRequest {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validated_pagination_replaces_both_default_values() {
        let query = KnowledgeListQuery::new()
            .try_with_pagination(PagePagination::try_new(3, u32::MAX).unwrap())
            .unwrap();
        assert_eq!(query.page, Some(3));
        assert_eq!(query.size, Some(u32::MAX));

        let default = KnowledgeListQuery::new();
        assert_eq!(default.page, Some(1));
        assert_eq!(default.size, Some(10));
    }
}
