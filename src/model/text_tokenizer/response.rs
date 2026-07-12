use serde::{Deserialize, Serialize};

/// Top-level response from the tokenizer endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenizerResponse {
    /// Unix timestamp (seconds) at which the result was created.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created: Option<i64>,
    /// Response id.
    pub id: String,
    /// Client-side request id, if one was supplied.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    /// Token-usage statistics for the input.
    pub usage: TokenizerUsage,
}

/// Token-usage statistics returned by the tokenizer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenizerUsage {
    /// Number of prompt tokens for the supplied messages.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_tokens: Option<f64>,
    /// Number of video tokens for the supplied messages.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub video_tokens: Option<f64>,
    /// Number of image tokens for the supplied messages.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_tokens: Option<f64>,
    /// Total number of tokens for the supplied messages.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_tokens: Option<f64>,
}

#[cfg(test)]
mod tests {
    use super::TokenizerResponse;

    #[test]
    fn accepts_optional_metadata_and_usage_fields() {
        let response: TokenizerResponse = serde_json::from_value(serde_json::json!({
            "id": "tokenizer-1",
            "usage": {}
        }))
        .expect("created, request_id, and usage properties are optional");

        assert_eq!(response.created, None);
        assert_eq!(response.usage.prompt_tokens, None);
        assert_eq!(response.usage.total_tokens, None);
    }

    #[test]
    fn rejects_missing_required_top_level_fields() {
        for value in [
            serde_json::json!({}),
            serde_json::json!({ "id": "tokenizer-1" }),
            serde_json::json!({ "usage": {} }),
        ] {
            assert!(serde_json::from_value::<TokenizerResponse>(value).is_err());
        }
    }
}
