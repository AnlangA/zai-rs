use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenizerResponse {
    pub created: i64,
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    pub usage: TokenizerUsage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenizerUsage {
    pub prompt_tokens: u64,
}

