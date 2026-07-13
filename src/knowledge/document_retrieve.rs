use super::types::DocumentGetResponse;
use crate::client::ZaiClient;

/// Retrieve document detail by id.
///
/// Credentials and transport live on the [`ZaiClient`], passed to
/// [`send_via`](Self::send_via).
pub struct DocumentGetRequest {
    document_id: String,
}

impl DocumentGetRequest {
    /// Create a new request targeting the given document id.
    pub fn new(document_id: impl Into<String>) -> Self {
        Self {
            document_id: document_id.into(),
        }
    }

    /// Send via a [`ZaiClient`] and parse the typed response.
    pub async fn send_via(&self, client: &ZaiClient) -> crate::ZaiResult<DocumentGetResponse> {
        crate::client::validation::require_non_blank(&self.document_id, "document_id")?;
        let route = crate::client::routes::DOCUMENTS_GET;
        client
            .operation(route)
            .with_parameters([self.document_id.as_str()])
            .send_empty::<DocumentGetResponse>()
            .await
    }
}
