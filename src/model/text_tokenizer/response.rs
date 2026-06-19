use serde::{Deserialize, Serialize};

/// Top-level response from the tokenizer endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenizerResponse {
    /// Unix timestamp (seconds) at which the result was created.
    pub created: i64,
    /// Response id.
    pub id: String,
    /// Client-side request id, if one was supplied.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    /// Token-usage statistics for the input.
    pub usage: TokenizerUsage,
}

/// Token-usage statistics returned by the tokenizer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenizerUsage {
    /// Number of prompt tokens for the supplied messages.
    pub prompt_tokens: u64,
}
