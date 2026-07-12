//! Semantic knowledge search (`POST /llm-application/open/knowledge/retrieve`).

use crate::ZaiResult;
use crate::client::ZaiClient;
use serde::Serialize;

/// JSON body for a semantic knowledge search.
#[derive(Debug, Clone, Serialize)]
pub struct KnowledgeSearchBody {
    /// Knowledge base id.
    pub knowledge_id: String,
    /// Search query text.
    pub query: String,
    /// Top-k results (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<u32>,
    /// Score threshold (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score_threshold: Option<f64>,
}

/// Request for semantic retrieval against one knowledge base.
///
/// This request does not perform local range validation for `top_k` or
/// `score_threshold`; configured values are sent unchanged.
pub struct KnowledgeSearchRequest {
    /// JSON body sent to the retrieval endpoint.
    pub body: KnowledgeSearchBody,
}

impl KnowledgeSearchRequest {
    /// Create a search request with the required knowledge-base ID and query.
    pub fn new(knowledge_id: impl Into<String>, query: impl Into<String>) -> Self {
        Self {
            body: KnowledgeSearchBody {
                knowledge_id: knowledge_id.into(),
                query: query.into(),
                top_k: None,
                score_threshold: None,
            },
        }
    }

    /// Set the requested maximum number of matches.
    pub fn with_top_k(mut self, top_k: u32) -> Self {
        self.body.top_k = Some(top_k);
        self
    }

    /// Set the minimum relevance score passed to the service.
    pub fn with_score_threshold(mut self, threshold: f64) -> Self {
        self.body.score_threshold = Some(threshold);
        self
    }

    /// Send the search request and decode its open-format result array.
    pub async fn send_via(&self, client: &ZaiClient) -> ZaiResult<KnowledgeSearchResponse> {
        let route = crate::client::routes::KNOWLEDGE_RETRIEVE;
        let url = client.endpoints().resolve_route(route, &[])?;
        client
            .send_json::<_, KnowledgeSearchResponse>(route.method(), url, &self.body)
            .await
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
/// Semantic search response with open-format result entries.
pub struct KnowledgeSearchResponse {
    /// Result objects returned by the service, preserved as JSON values.
    #[serde(default)]
    pub data: Vec<serde_json::Value>,
}
