use serde::{Deserialize, Serialize};

/// Top-level response from the rerank endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RerankResponse {
    /// Unix timestamp (seconds) at which the result was created.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created: Option<i64>,
    /// Response id.
    pub id: String,
    /// Client-side request id, if one was supplied.
    #[serde(skip_serializing_if = "Option::is_none")]
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
    /// Document text returned for this result.
    pub document: String,
}

/// Token-usage statistics for a rerank request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RerankUsage {
    /// Number of prompt tokens.
    pub prompt_tokens: u64,
    /// Total tokens for this request.
    pub total_tokens: u64,
}

#[cfg(test)]
mod tests {
    use super::RerankResponse;

    #[test]
    fn accepts_optional_top_level_metadata() {
        let response: RerankResponse = serde_json::from_value(serde_json::json!({
            "id": "rerank-1",
            "results": [{
                "document": "first",
                "index": 0,
                "relevance_score": 0.9
            }],
            "usage": {
                "prompt_tokens": 2,
                "total_tokens": 2
            }
        }))
        .expect("created and request_id are optional");

        assert_eq!(response.created, None);
        assert_eq!(response.results[0].document, "first");
    }

    #[test]
    fn rejects_missing_required_fields() {
        for value in [
            serde_json::json!({}),
            serde_json::json!({
                "id": "rerank-1",
                "results": [],
                "usage": { "prompt_tokens": 1 }
            }),
            serde_json::json!({
                "id": "rerank-1",
                "results": [{ "index": 0, "relevance_score": 0.9 }],
                "usage": { "prompt_tokens": 1, "total_tokens": 1 }
            }),
        ] {
            assert!(serde_json::from_value::<RerankResponse>(value).is_err());
        }
    }
}
