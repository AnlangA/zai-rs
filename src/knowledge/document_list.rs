use super::types::DocumentListResponse;
use crate::ZaiResult;
use crate::client::ZaiClient;

/// Query parameters for listing documents under a knowledge base
#[derive(Debug, Clone, serde::Serialize, validator::Validate, Default)]
#[allow(clippy::new_without_default)]
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

    /// Build the `(&str, String)` query pairs (in stable order) used to form
    /// the request URL.
    fn pairs(&self) -> Vec<(&'static str, String)> {
        let mut params: Vec<(&'static str, String)> = Vec::new();
        params.push(("knowledge_id", self.knowledge_id.clone()));
        if let Some(page) = self.page.as_ref() {
            params.push(("page", page.to_string()));
        }
        if let Some(size) = self.size.as_ref() {
            params.push(("size", size.to_string()));
        }
        if let Some(word) = self.word.as_ref() {
            params.push(("word", word.clone()));
        }
        params
    }
}

/// Document list request (GET /llm-application/open/document)
///
/// Credentials and transport live on the [`ZaiClient`], passed to
/// [`send_via`](Self::send_via).
#[allow(clippy::new_without_default)]
pub struct DocumentListRequest {
    query: Option<DocumentListQuery>,
}

impl DocumentListRequest {
    /// Create a new document-list request (no query set yet).
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self { query: None }
    }

    /// Apply a query (replaces the current one).
    pub fn with_query(mut self, q: DocumentListQuery) -> Self {
        self.query = Some(q);
        self
    }

    /// Send via a [`ZaiClient`] and parse the typed response. Requires a query
    /// to have been set via [`with_query`](Self::with_query).
    pub async fn send_via(&self, client: &ZaiClient) -> ZaiResult<DocumentListResponse> {
        let q = self
            .query
            .as_ref()
            .ok_or_else(|| crate::ZaiError::ApiError {
                code: crate::client::error::codes::SDK_VALIDATION,
                message: "document list requires a knowledge_id; call with_query first".to_string(),
            })?;
        let params = q.pairs();
        let url = client.endpoints().resolve_with_query(
            crate::client::ApiFamily::LlmApplication,
            &["document"],
            &params
                .iter()
                .map(|(k, v)| (*k, v.as_str()))
                .collect::<Vec<_>>(),
        )?;
        client.send_empty::<DocumentListResponse>("GET", url).await
    }

    /// Validate the query then send via a [`ZaiClient`] and parse the typed
    /// response.
    pub async fn send_via_with_query(
        mut self,
        client: &ZaiClient,
        q: &DocumentListQuery,
    ) -> ZaiResult<DocumentListResponse> {
        use validator::Validate;
        q.validate()?;
        self.query = Some(q.clone());
        self.send_via(client).await
    }
}
