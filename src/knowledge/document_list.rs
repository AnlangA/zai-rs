use super::types::DocumentListResponse;
use crate::ZaiResult;
use crate::client::ZaiClient;

/// Query parameters for listing documents under a knowledge base
#[derive(Clone, serde::Serialize, validator::Validate)]
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

impl std::fmt::Debug for DocumentListQuery {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("DocumentListQuery")
            .field("knowledge_id", &"[REDACTED]")
            .field("page", &self.page)
            .field("size", &self.size)
            .field("word", &self.word.as_ref().map(|_| "[REDACTED]"))
            .finish()
    }
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
pub struct DocumentListRequest {
    query: DocumentListQuery,
}

impl DocumentListRequest {
    /// Create a document-list request for the required knowledge-base id.
    pub fn new(knowledge_id: impl Into<String>) -> Self {
        Self {
            query: DocumentListQuery::new(knowledge_id),
        }
    }

    /// Apply a query (replaces the current one).
    pub fn with_query(mut self, q: DocumentListQuery) -> Self {
        self.query = q;
        self
    }

    /// Set the one-based page index.
    pub fn with_page(mut self, page: u32) -> Self {
        self.query.page = Some(page);
        self
    }

    /// Set the requested page size.
    pub fn with_size(mut self, size: u32) -> Self {
        self.query.size = Some(size);
        self
    }

    /// Filter documents by name.
    pub fn with_word(mut self, word: impl Into<String>) -> Self {
        self.query.word = Some(word.into());
        self
    }

    /// Validate the configured query, send it, and parse the typed response.
    pub async fn send_via(&self, client: &ZaiClient) -> ZaiResult<DocumentListResponse> {
        use validator::Validate;
        self.query.validate()?;
        crate::client::validation::require_non_blank(&self.query.knowledge_id, "knowledge_id")?;
        if self
            .query
            .word
            .as_deref()
            .is_some_and(|word| word.trim().is_empty())
        {
            return Err(crate::client::validation::invalid(
                "word must not be blank when provided",
            ));
        }
        let params = self.query.pairs();
        let route = crate::client::routes::DOCUMENTS_LIST;
        client
            .operation(route)
            .with_query(params)
            .send_empty::<DocumentListResponse>()
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_debug_redacts_knowledge_id_and_word() {
        let query = DocumentListQuery::new("private-knowledge").with_word("private-document");
        let debug = format!("{query:?}");
        assert!(!debug.contains("private-knowledge"));
        assert!(!debug.contains("private-document"));
    }
}
