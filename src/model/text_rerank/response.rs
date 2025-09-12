use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RerankResponse {
    pub created: i64,
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    pub results: Vec<RerankResult>,
    pub usage: RerankUsage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RerankResult {
    pub index: usize,
    pub relevance_score: f32,
    /// Present only when return_documents=true
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub document: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RerankUsage {
    pub prompt_tokens: u64,
    #[serde(default)]
    pub total_tokens: u64,
}
