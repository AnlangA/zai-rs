use super::types::KnowledgeCapacityResponse;
use crate::client::ZaiClient;

/// Knowledge capacity request (GET /llm-application/open/knowledge/capacity)
///
/// Credentials and transport live on the [`ZaiClient`], passed to
/// [`send_via`](Self::send_via).
#[allow(clippy::new_without_default)]
#[derive(Default)]
pub struct KnowledgeCapacityRequest {
    _body: (),
}

impl KnowledgeCapacityRequest {
    /// Build a capacity request.
    pub fn new() -> Self {
        Self { _body: () }
    }

    /// Send via a [`ZaiClient`] and parse the typed response.
    pub async fn send_via(
        &self,
        client: &ZaiClient,
    ) -> crate::ZaiResult<KnowledgeCapacityResponse> {
        let url = client.endpoints().resolve(
            crate::client::ApiFamily::LlmApplication,
            &["knowledge", "capacity"],
        )?;
        client
            .send_empty::<KnowledgeCapacityResponse>("GET", url)
            .await
    }
}
