use super::types::KnowledgeCapacityResponse;
use crate::client::ZaiClient;

/// Knowledge capacity request (GET /llm-application/open/knowledge/capacity)
///
/// Credentials and transport live on the [`ZaiClient`], passed to
/// [`send_via`](Self::send_via).
#[derive(Default)]
pub struct KnowledgeCapacityRequest;

impl KnowledgeCapacityRequest {
    /// Build a capacity request.
    pub fn new() -> Self {
        Self
    }

    /// Send via a [`ZaiClient`] and parse the typed response.
    pub async fn send_via(
        &self,
        client: &ZaiClient,
    ) -> crate::ZaiResult<KnowledgeCapacityResponse> {
        let route = crate::client::routes::KNOWLEDGE_CAPACITY;
        let url = client.endpoints().resolve_route(route, &[])?;
        client
            .send_empty::<KnowledgeCapacityResponse>(route.method(), url)
            .await
    }
}
