use super::{
    request::{RerankBody, RerankModel},
    response::RerankResponse,
};
use crate::ZaiResult;
use crate::client::ZaiClient;

/// Text Rerank request client (JSON POST)
///
/// Builder for the rerank endpoint. Construct with [`RerankRequest::new`],
/// tune with the `with_*` methods, then call [`RerankRequest::send_via`].
pub struct RerankRequest {
    body: RerankBody,
}

impl RerankRequest {
    /// Create a new rerank request for a query and a set of candidate
    /// documents.
    pub fn new(query: impl Into<String>, documents: Vec<String>) -> Self {
        let body = RerankBody::new(RerankModel::Rerank, query, documents);
        Self { body }
    }

    /// Set how many top-ranked documents to return.
    pub fn with_top_n(mut self, n: usize) -> Self {
        self.body = self.body.with_top_n(n);
        self
    }
    /// Whether to include the document text in the response.
    pub fn with_return_documents(mut self, v: bool) -> Self {
        self.body = self.body.with_return_documents(v);
        self
    }
    /// Whether to include raw relevance scores in the response.
    pub fn with_return_raw_scores(mut self, v: bool) -> Self {
        self.body = self.body.with_return_raw_scores(v);
        self
    }
    /// Set the client-side request id.
    pub fn with_request_id(mut self, v: impl Into<String>) -> Self {
        self.body = self.body.with_request_id(v);
        self
    }
    /// Set the end-user id.
    pub fn with_user_id(mut self, v: impl Into<String>) -> Self {
        self.body = self.body.with_user_id(v);
        self
    }

    /// Validate query, document, and result-count constraints before sending.
    pub fn validate(&self) -> ZaiResult<()> {
        self.body.validate_constraints()
    }

    /// Send via a [`ZaiClient`] and parse the typed response.
    /// Automatically runs `validate()` before sending.
    pub async fn send_via(&self, client: &ZaiClient) -> ZaiResult<RerankResponse> {
        self.validate()?;
        let route = crate::client::routes::RERANK_CREATE;
        client
            .operation(route)
            .send_json::<_, RerankResponse>(&self.body)
            .await
    }
}
