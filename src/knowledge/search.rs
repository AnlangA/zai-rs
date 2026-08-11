//! Semantic knowledge search (`POST /llm-application/open/knowledge/retrieve`).

use serde::{Deserialize, Deserializer, Serialize, de::Error as _};
use validator::Validate;

use crate::{ZaiResult, client::ZaiClient};

/// Retrieval strategy used before optional reranking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum KnowledgeRecallMethod {
    /// Vector similarity search.
    Embedding,
    /// Keyword search.
    Keyword,
    /// Combine vector and keyword search.
    Mixed,
}

/// Reranking model applied to recalled chunks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KnowledgeRerankModel {
    /// Standard reranker.
    #[serde(rename = "rerank")]
    Rerank,
    /// Higher-quality reranker.
    #[serde(rename = "rerank-pro")]
    RerankPro,
}

/// JSON body for a semantic knowledge search.
#[derive(Clone, Serialize, Deserialize, Validate)]
pub struct KnowledgeSearchBody {
    /// Optional caller-generated request identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    /// Search query text (up to 1000 characters).
    #[validate(length(min = 1, max = 1000))]
    pub query: String,
    /// Knowledge bases to search.
    #[validate(length(min = 1))]
    pub knowledge_ids: Vec<String>,
    /// Restrict retrieval to these document identifiers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document_ids: Option<Vec<String>>,
    /// Final result count (`1..=20`, server default `8`).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(range(min = 1, max = 20))]
    pub top_k: Option<u32>,
    /// Initial recall count (`1..=100`, server default `10`).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(range(min = 1, max = 100))]
    pub top_n: Option<u32>,
    /// Recall strategy (server default: mixed).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recall_method: Option<KnowledgeRecallMethod>,
    /// Vector-search weight for mixed retrieval (`1..=99`).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(range(min = 1, max = 99))]
    pub recall_ratio: Option<u8>,
    /// Whether reranking is enabled (`0` or `1`).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(range(max = 1))]
    pub rerank_status: Option<u8>,
    /// Reranking model.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rerank_model: Option<KnowledgeRerankModel>,
    /// Similarity threshold in the open interval `(0, 1)`.
    ///
    /// The Rust field keeps its historical name while the wire field follows
    /// the upstream `fractional_threshold` schema.
    #[serde(
        rename = "fractional_threshold",
        skip_serializing_if = "Option::is_none"
    )]
    pub score_threshold: Option<f64>,
}

impl std::fmt::Debug for KnowledgeSearchBody {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("KnowledgeSearchBody")
            .field(
                "request_id",
                &self.request_id.as_ref().map(|_| "[REDACTED]"),
            )
            .field("query", &"[REDACTED]")
            .field("knowledge_id_count", &self.knowledge_ids.len())
            .field(
                "document_id_count",
                &self.document_ids.as_ref().map(Vec::len),
            )
            .field("top_k", &self.top_k)
            .field("top_n", &self.top_n)
            .field("recall_method", &self.recall_method)
            .field("recall_ratio", &self.recall_ratio)
            .field("rerank_status", &self.rerank_status)
            .field("rerank_model", &self.rerank_model)
            .field("score_threshold", &self.score_threshold)
            .finish()
    }
}

/// Request for semantic retrieval across one or more knowledge bases.
pub struct KnowledgeSearchRequest {
    /// JSON body sent to the retrieval endpoint.
    pub body: KnowledgeSearchBody,
}

impl KnowledgeSearchRequest {
    /// Create a search request targeting one knowledge base.
    pub fn new(knowledge_id: impl Into<String>, query: impl Into<String>) -> Self {
        Self {
            body: KnowledgeSearchBody {
                request_id: None,
                query: query.into(),
                knowledge_ids: vec![knowledge_id.into()],
                document_ids: None,
                top_k: None,
                top_n: None,
                recall_method: None,
                recall_ratio: None,
                rerank_status: None,
                rerank_model: None,
                score_threshold: None,
            },
        }
    }

    /// Replace the knowledge-base selection.
    pub fn with_knowledge_ids(mut self, knowledge_ids: Vec<String>) -> Self {
        self.body.knowledge_ids = knowledge_ids;
        self
    }

    /// Restrict retrieval to selected documents.
    pub fn with_document_ids(mut self, document_ids: Vec<String>) -> Self {
        self.body.document_ids = Some(document_ids);
        self
    }

    /// Set a caller-generated request identifier.
    pub fn with_request_id(mut self, request_id: impl Into<String>) -> Self {
        self.body.request_id = Some(request_id.into());
        self
    }

    /// Set the requested maximum number of final matches.
    pub fn with_top_k(mut self, top_k: u32) -> Self {
        self.body.top_k = Some(top_k);
        self
    }

    /// Set the number of candidates recalled before reranking.
    pub fn with_top_n(mut self, top_n: u32) -> Self {
        self.body.top_n = Some(top_n);
        self
    }

    /// Select the retrieval strategy.
    pub fn with_recall_method(mut self, method: KnowledgeRecallMethod) -> Self {
        self.body.recall_method = Some(method);
        self
    }

    /// Set the vector-search weight for mixed retrieval.
    pub fn with_recall_ratio(mut self, ratio: u8) -> Self {
        self.body.recall_ratio = Some(ratio);
        self
    }

    /// Configure reranking and its model.
    pub fn with_reranking(mut self, enabled: bool, model: KnowledgeRerankModel) -> Self {
        self.body.rerank_status = Some(u8::from(enabled));
        self.body.rerank_model = Some(model);
        self
    }

    /// Set the minimum relevance score sent as `fractional_threshold`.
    pub fn with_score_threshold(mut self, threshold: f64) -> Self {
        self.body.score_threshold = Some(threshold);
        self
    }

    /// Validate and send the search request.
    pub async fn send_via(&self, client: &ZaiClient) -> ZaiResult<KnowledgeSearchResponse> {
        self.body.validate()?;
        if self.body.query.trim().is_empty() {
            return Err(crate::client::validation::invalid("query cannot be blank"));
        }
        if self
            .body
            .knowledge_ids
            .iter()
            .any(|knowledge_id| knowledge_id.trim().is_empty())
        {
            return Err(crate::client::validation::invalid(
                "knowledge_ids cannot contain blank values",
            ));
        }
        if self
            .body
            .document_ids
            .as_ref()
            .is_some_and(|ids| ids.is_empty() || ids.iter().any(|id| id.trim().is_empty()))
        {
            return Err(crate::client::validation::invalid(
                "document_ids must contain at least one non-blank value",
            ));
        }
        if self
            .body
            .score_threshold
            .is_some_and(|threshold| !threshold.is_finite() || threshold <= 0.0 || threshold >= 1.0)
        {
            return Err(crate::client::validation::invalid(
                "score_threshold must be finite and in the range 0.0 < value < 1.0",
            ));
        }

        let route = crate::client::routes::KNOWLEDGE_RETRIEVE;
        client
            .operation(route)
            .send_json::<_, KnowledgeSearchResponse>(&self.body)
            .await
    }
}

/// Metadata attached to one retrieved knowledge chunk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeSearchMetadata {
    /// Chunk identifier.
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Knowledge-base identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub knowledge_id: Option<String>,
    /// Source document identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doc_id: Option<String>,
    /// Source document name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doc_name: Option<String>,
    /// Source document URL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doc_url: Option<String>,
    /// Context-enriched text, when contextual retrieval was enabled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contextual_text: Option<String>,
}

/// One semantic-retrieval result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeSearchResult {
    /// Retrieved chunk text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// Similarity score reported by the service.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score: Option<f64>,
    /// Structured source metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<KnowledgeSearchMetadata>,
}

/// Semantic search success response.
///
/// The frozen knowledge contract requires business code `200` and a non-null
/// `data` array. The fields remain optional for source compatibility, while
/// deserialization rejects partial HTTP-200 envelopes.
#[derive(Debug, Clone, Serialize)]
pub struct KnowledgeSearchResponse {
    /// Retrieved chunk objects. Successful deserialization requires this field
    /// to be present and non-null; an empty array is valid.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Vec<KnowledgeSearchResult>>,
    /// Business status code. Successful deserialization requires exactly
    /// `200`; the shared transport also rejects explicit business failures.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<i64>,
    /// Human-readable status message.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// Server timestamp.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<u64>,
}

#[derive(Deserialize)]
struct KnowledgeSearchResponseWire {
    data: Option<Vec<KnowledgeSearchResult>>,
    code: Option<i64>,
    message: Option<String>,
    timestamp: Option<u64>,
}

impl<'de> Deserialize<'de> for KnowledgeSearchResponse {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = KnowledgeSearchResponseWire::deserialize(deserializer)?;
        if wire.code != Some(200) {
            return Err(D::Error::custom(
                "knowledge-search response must contain business code 200",
            ));
        }
        if wire.data.is_none() {
            return Err(D::Error::custom(
                "knowledge-search response must contain non-null data",
            ));
        }
        Ok(Self {
            data: wire.data,
            code: wire.code,
            message: wire.message,
            timestamp: wire.timestamp,
        })
    }
}

impl KnowledgeSearchResponse {
    /// Borrow the returned results when the service included them.
    pub fn results(&self) -> Option<&[KnowledgeSearchResult]> {
        self.data.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn convenience_builder_matches_official_wire_names() {
        let request = KnowledgeSearchRequest::new("kb-1", "question")
            .with_top_k(5)
            .with_score_threshold(0.25);
        let value = serde_json::to_value(&request.body).unwrap();

        assert_eq!(value["knowledge_ids"], serde_json::json!(["kb-1"]));
        assert_eq!(value["fractional_threshold"], 0.25);
        assert!(value.get("knowledge_id").is_none());
        assert!(value.get("score_threshold").is_none());
    }

    #[test]
    fn request_body_debug_redacts_query_and_identifiers() {
        let body = KnowledgeSearchRequest::new("private-knowledge", "private query")
            .with_document_ids(vec!["private-document".to_owned()])
            .with_request_id("private-request")
            .body;
        let debug = format!("{body:?}");
        for secret in [
            "private-knowledge",
            "private query",
            "private-document",
            "private-request",
        ] {
            assert!(!debug.contains(secret));
        }
        assert!(debug.contains("knowledge_id_count: 1"));
        assert!(debug.contains("document_id_count: Some(1)"));
    }

    #[test]
    fn response_uses_typed_metadata_and_requires_complete_success_envelope() {
        let response: KnowledgeSearchResponse = serde_json::from_value(serde_json::json!({
            "code": 200,
            "data": [{
                "text": "chunk",
                "score": 0.9,
                "metadata": {
                    "_id": "slice-1",
                    "knowledge_id": "kb-1",
                    "doc_id": "doc-1",
                    "doc_name": "guide.md",
                    "doc_url": "https://example.com/guide.md",
                    "contextual_text": "context"
                }
            }]
        }))
        .unwrap();
        let result = &response.results().unwrap()[0];
        assert_eq!(result.text.as_deref(), Some("chunk"));
        assert_eq!(result.score, Some(0.9));
        assert_eq!(
            result
                .metadata
                .as_ref()
                .and_then(|metadata| metadata.id.as_deref()),
            Some("slice-1")
        );

        assert!(
            serde_json::from_value::<KnowledgeSearchResponse>(
                serde_json::json!({"code": 200, "data": []})
            )
            .is_ok()
        );
        for invalid in [
            serde_json::json!({"data": []}),
            serde_json::json!({"code": 200}),
            serde_json::json!({"code": 0, "data": []}),
            serde_json::json!({"code": 200, "data": null}),
            serde_json::json!({}),
        ] {
            assert!(serde_json::from_value::<KnowledgeSearchResponse>(invalid).is_err());
        }
    }
}
