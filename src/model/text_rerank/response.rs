use serde::{Deserialize, Serialize};

/// Top-level response from the rerank endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RerankResponse {
    /// Unix timestamp (seconds) at which the result was created.
    pub created: i64,
    /// Response id.
    pub id: String,
    /// Client-side request id, if one was supplied.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    /// Ranked results (best first).
    pub results: Vec<RerankResult>,
    /// Token-usage statistics for the request.
    pub usage: RerankUsage,
}

/// A single ranked document with its relevance score.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RerankResult {
    /// Index of this document within the original `documents` input.
    pub index: usize,
    /// Relevance score for this document.
    pub relevance_score: f32,
    /// Present only when return_documents=true
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub document: Option<String>,
}

/// Token-usage statistics for a rerank request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RerankUsage {
    /// Number of prompt tokens.
    pub prompt_tokens: u64,
    /// Total tokens for this request.
    #[serde(default)]
    pub total_tokens: u64,
}
