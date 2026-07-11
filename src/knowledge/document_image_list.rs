use super::types::DocumentImageListResponse;
use crate::client::ZaiClient;

/// Retrieve parsed image index-url mapping for a document (POST, no body)
///
/// Credentials and transport live on the [`ZaiClient`], passed to
/// [`send_via`](Self::send_via).
pub struct DocumentImageListRequest {
    document_id: String,
}

impl DocumentImageListRequest {
    /// Create a new request with the target document id
    pub fn new(document_id: impl Into<String>) -> Self {
        Self {
            document_id: document_id.into(),
        }
    }

    /// Send the POST request via a [`ZaiClient`] and parse the typed response.
    pub async fn send_via(
        &self,
        client: &ZaiClient,
    ) -> crate::ZaiResult<DocumentImageListResponse> {
        let route = crate::client::routes::DOCUMENTS_IMAGES;
        let url = client
            .endpoints()
            .resolve_route(route, &[&self.document_id])?;
        client
            .send_empty::<DocumentImageListResponse>(route.method(), url)
            .await
    }
}
