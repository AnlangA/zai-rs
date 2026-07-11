use crate::client::ZaiClient;

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
        let route = crate::client::routes::DOCUMENTS_DELETE;
        let url = client.endpoints().resolve_route(route, &[&self.id])?;
        client
            .send_empty::<DocumentDeleteResponse>(route.method(), url)
            .await
    }
}

/// Delete response envelope without data
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, validator::Validate)]
pub struct DocumentDeleteResponse {
    /// Business status code.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<i64>,
    /// Human-readable message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// Server timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<u64>,
}
