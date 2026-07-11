//! Knowledge search (POST /llm-application/open/knowledge/retrieve) — plan P06.

use crate::ZaiResult;
use crate::client::{ApiFamily, ZaiClient};
use serde::Serialize;

/// POST /knowledge/retrieve search request (P06 knowledge.retrieve).
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

pub struct KnowledgeSearchRequest {
    pub body: KnowledgeSearchBody,
}

impl KnowledgeSearchRequest {
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

    pub fn with_top_k(mut self, top_k: u32) -> Self {
        self.body.top_k = Some(top_k);
        self
    }

    pub fn with_score_threshold(mut self, threshold: f64) -> Self {
        self.body.score_threshold = Some(threshold);
        self
    }

    pub async fn send_via(&self, client: &ZaiClient) -> ZaiResult<KnowledgeSearchResponse> {
        let url = client
            .endpoints()
            .resolve(ApiFamily::LlmApplication, &["knowledge", "retrieve"])?;
        client
            .send_json::<_, KnowledgeSearchResponse>("POST", url, &self.body)
            .await
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct KnowledgeSearchResponse {
    #[serde(default)]
    pub data: Vec<serde_json::Value>,
}
