use super::types::KnowledgeDetailResponse;
use crate::client::ZaiClient;

/// Knowledge detail request (GET /llm-application/open/knowledge/{id})
///
/// Credentials and transport live on the [`ZaiClient`], passed to
/// [`send_via`](Self::send_via).
pub struct KnowledgeRetrieveRequest {
    id: String,
}

impl KnowledgeRetrieveRequest {
    /// Build a retrieve request with id.
    pub fn new(id: impl Into<String>) -> Self {
        Self { id: id.into() }
    }

    /// Send via a [`ZaiClient`] and parse the typed response.
    pub async fn send_via(&self, client: &ZaiClient) -> crate::ZaiResult<KnowledgeDetailResponse> {
        let route = crate::client::routes::KNOWLEDGE_GET;
        let url = client.endpoints().resolve_route(route, &[&self.id])?;
        client
            .send_empty::<KnowledgeDetailResponse>(route.method(), url)
            .await
    }
}

/// Alias for symmetry with other modules
pub type KnowledgeRetrieveResponse = KnowledgeDetailResponse;
