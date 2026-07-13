use crate::client::ZaiClient;

use super::types::KnowledgeOperationResponse;

/// Document delete request (DELETE /llm-application/open/document/{id})
///
/// Credentials and transport live on the [`ZaiClient`], passed to
/// [`send_via`](Self::send_via).
pub struct DocumentDeleteRequest {
    id: String,
}

impl DocumentDeleteRequest {
    /// Build a delete request with target document id.
    pub fn new(id: impl Into<String>) -> Self {
        Self { id: id.into() }
    }

    /// Send via a [`ZaiClient`] and parse the typed response.
    pub async fn send_via(&self, client: &ZaiClient) -> crate::ZaiResult<DocumentDeleteResponse> {
        crate::client::validation::require_non_blank(&self.id, "document_id")?;
        let route = crate::client::routes::DOCUMENTS_DELETE;
        client
            .operation(route)
            .with_parameters([self.id.as_str()])
            .send_empty::<DocumentDeleteResponse>()
            .await
    }
}

/// Delete response envelope without data.
pub type DocumentDeleteResponse = KnowledgeOperationResponse;
