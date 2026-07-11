use crate::client::ZaiClient;

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
        let route = crate::client::routes::KNOWLEDGE_DELETE;
        let url = client.endpoints().resolve_route(route, &[&self.id])?;
        client
            .send_empty::<KnowledgeDeleteResponse>(route.method(), url)
            .await
    }
}

/// Delete response envelope without data
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, validator::Validate)]
pub struct KnowledgeDeleteResponse {
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
