//! Streaming response types for chat API models.
//!
//! These types mirror ChatCompletionResponse but are tailored for streaming deltas
//! (data: lines), where choices contain `delta` instead of a full `message`.

use serde::{Deserialize, Deserializer, Serialize};

/// Helper: accept string or number and always deserialize into Option<String>
fn de_opt_string_from_number_or_string<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let v = serde_json::Value::deserialize(deserializer)?;
    match v {
        serde_json::Value::Null => Ok(None),
        serde_json::Value::String(s) => Ok(Some(s)),
        serde_json::Value::Number(n) => Ok(Some(n.to_string())),
        other => Err(serde::de::Error::custom(format!(
            "expected string or number, got {}",
            other
        ))),
    }
}

/// One streaming chunk (single data: line) from the server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatStreamResponse {
    /// Task ID (string or number on wire)
    #[serde(
        skip_serializing_if = "Option::is_none",
        deserialize_with = "de_opt_string_from_number_or_string"
    )]
    pub id: Option<String>,

    /// Created time, unix seconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created: Option<u64>,

    /// Model name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    /// Streaming choices (usually length 1 per chunk)
    pub choices: Vec<StreamChoice>,

    /// Usage appears on the final chunk for some providers
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<crate::model::chat_base_response::Usage>,
}

/// One choice item in a streaming chunk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChoice {
    /// Index of this result
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<i32>,

    /// Delta payload with partial content
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delta: Option<Delta>,

    /// Why generation finished (typically on final chunk)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
}

/// Delta payload for streaming content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Delta {
    /// Role of the message (assistant on the first chunk)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,

    /// Partial text content
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,

    /// Reasoning traces (when available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_content: Option<String>,
}
