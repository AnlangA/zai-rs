use super::types::KnowledgeGetResponse;
use crate::client::ZaiClient;

/// Knowledge detail request (GET /llm-application/open/knowledge/{id})
///
/// Credentials and transport live on the [`ZaiClient`], passed to
/// [`send_via`](Self::send_via).
pub struct KnowledgeGetRequest {
    id: String,
}

impl KnowledgeGetRequest {
    /// Build a retrieve request with id.
    pub fn new(id: impl Into<String>) -> Self {
        Self { id: id.into() }
    }

    /// Send via a [`ZaiClient`] and parse the typed response.
    pub async fn send_via(&self, client: &ZaiClient) -> crate::ZaiResult<KnowledgeGetResponse> {
        crate::client::validation::require_non_blank(&self.id, "knowledge_id")?;
        let route = crate::client::routes::KNOWLEDGE_GET;
        client
            .operation(route)
            .with_parameters([self.id.as_str()])
            .send_empty::<KnowledgeGetResponse>()
            .await
    }
}
