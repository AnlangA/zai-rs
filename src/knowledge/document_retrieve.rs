use super::types::DocumentDetailResponse;
use crate::client::ZaiClient;

/// Retrieve document detail by id.
///
/// Credentials and transport live on the [`ZaiClient`], passed to
/// [`send_via`](Self::send_via).
pub struct DocumentRetrieveRequest {
    document_id: String,
}

impl DocumentRetrieveRequest {
    /// Create a new request targeting the given document id.
    pub fn new(document_id: impl Into<String>) -> Self {
        Self {
            document_id: document_id.into(),
        }
    }

    /// Send via a [`ZaiClient`] and parse the typed response.
    pub async fn send_via(&self, client: &ZaiClient) -> crate::ZaiResult<DocumentDetailResponse> {
        let url = client.endpoints().resolve(
            crate::client::ApiFamily::LlmApplication,
            &["document", &self.document_id],
        )?;
        client
            .send_empty::<DocumentDetailResponse>("GET", url)
            .await
    }
}
