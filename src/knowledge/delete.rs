use crate::client::ZaiClient;

use super::types::KnowledgeOperationResponse;

/// Knowledge delete request (DELETE /llm-application/open/knowledge/{id})
///
/// Credentials and transport live on the [`ZaiClient`], passed to
/// [`send_via`](Self::send_via).
pub struct KnowledgeDeleteRequest {
    id: String,
}

impl KnowledgeDeleteRequest {
    /// Build a delete request with target id.
    pub fn new(id: impl Into<String>) -> Self {
        Self { id: id.into() }
    }

    /// Send via a [`ZaiClient`] and parse the typed response.
    pub async fn send_via(&self, client: &ZaiClient) -> crate::ZaiResult<KnowledgeDeleteResponse> {
        crate::client::validation::require_non_blank(&self.id, "knowledge_id")?;
        let route = crate::client::routes::KNOWLEDGE_DELETE;
        client
            .operation(route)
            .with_parameters([self.id.as_str()])
            .send_empty::<KnowledgeDeleteResponse>()
            .await
    }
}

/// Delete response envelope without data.
pub type KnowledgeDeleteResponse = KnowledgeOperationResponse;
