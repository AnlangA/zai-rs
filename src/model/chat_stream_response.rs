//! # Streaming Response Types for Chat API Models
//!
//! This module defines the data structures used for processing streaming
//! responses from chat completion APIs. These types are specifically designed
//! to handle Server-Sent Events (SSE) data chunks where responses arrive
//! incrementally.
//!
//! ## Key Differences from Standard Responses
//!
//! Unlike regular chat completion responses, streaming responses:
//! - Contain `delta` fields instead of complete `message` objects
//! - Arrive as multiple chunks over time
//! - Include partial content that gets assembled client-side
//! - May contain reasoning content for models with thinking capabilities
//!
//! ## Streaming Protocol
//!
//! The streaming implementation expects SSE-formatted data with:
//! - `data: ` prefixed lines containing JSON chunks
//! - `[DONE]` marker to signal stream completion
//! - Optional usage statistics on the final chunk
//!
//! ## Decoding a chunk
//!
//! ```
//! use zai_rs::model::chat_stream_response::ChatStreamResponse;
//!
//! let chunk: ChatStreamResponse = serde_json::from_str(
//!     r#"{"id":null,"choices":[{"index":0,"delta":{"content":"Hello"}}]}"#,
//! )?;
//! assert_eq!(chunk.choices[0].delta.as_ref().unwrap().content.as_deref(), Some("Hello"));
//! # Ok::<(), serde_json::Error>(())
//! ```

use serde::{Deserialize, Serialize};

/// Represents a single streaming chunk from the chat API.
///
/// This struct contains a portion of the complete response that arrives
/// as part of an SSE stream. Multiple chunks are typically received
/// and assembled to form the complete response.
///
/// ## Fields
///
/// - `id` - Unique identifier for the streaming session (optional)
/// - `created` - Unix timestamp when the chunk was created (optional)
/// - `model` - Name of the model generating the response (optional)
/// - `choices` - Array of streaming choices, usually containing one item
/// - `usage` - Token usage statistics, typically only on final chunk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatStreamResponse {
    /// Unique identifier for the streaming session.
    ///
    /// May be a string or number in the wire format, converted to
    /// `Option<String>`.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "super::serde_helpers::optional_string_from_number_or_string"
    )]
    pub id: Option<String>,

    /// Unix timestamp indicating when the chunk was created.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created: Option<u64>,

    /// Name of the AI model generating the response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    /// Array of streaming choices, typically containing one item per chunk.
    ///
    /// Each choice contains a delta with partial content updates.
    pub choices: Vec<StreamChoice>,

    /// Token usage statistics.
    ///
    /// This field typically appears only on the final chunk of the stream,
    /// providing information about prompt and completion token counts.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<crate::model::chat_base_response::Usage>,
}

/// Represents a single choice within a streaming response chunk.
///
/// Each choice contains a delta with incremental content updates and
/// metadata about the generation process.
///
/// ## Fields
///
/// - `index` - Position of this choice in the results array
/// - `delta` - Partial content update for this choice
/// - `finish_reason` - Reason why generation stopped (on final chunk)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChoice {
    /// Index position of this choice in the results array.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<i32>,

    /// Delta payload containing partial content updates.
    ///
    /// This field contains the incremental content that should be
    /// appended to the accumulated response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delta: Option<Delta>,

    /// Reason why the generation process finished.
    ///
    /// This field typically appears only on the final chunk of a choice,
    /// indicating why generation stopped (e.g., "stop", "length", etc.).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
}

/// Represents incremental content updates in streaming responses.
///
/// The delta contains partial content that should be appended to the
/// accumulated response. Different fields may be present depending on
/// the chunk type and model capabilities.
///
/// ## Fields
///
/// - `role` - Message role, typically "assistant" on first chunk
/// - `content` - Partial text content to append
/// - `reasoning_content` - Reasoning traces for thinking models
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Delta {
    /// Role of the message sender.
    ///
    /// Typically "assistant" on the first chunk of a response,
    /// may be omitted on subsequent chunks.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,

    /// Partial text content that should be appended to the response.
    ///
    /// This field contains the incremental text content for the current chunk.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,

    /// Reasoning content for models with thinking capabilities.
    ///
    /// This field contains step-by-step reasoning traces when the model
    /// is operating in thinking mode with reasoning enabled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_content: Option<String>,

    /// Streaming tool call payload for tool invocation.
    ///
    /// When `tool_stream` is enabled, the provider emits partially populated
    /// tool calls whose names and JSON-string arguments can arrive in separate
    /// chunks.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<StreamToolCall>>,
}

/// One incremental function call in a streaming delta.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamToolCall {
    /// Position of this call in the assistant's tool-call list.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<i32>,
    /// Tool-call identifier, usually present in the first delta for a call.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Tool kind; the frozen streaming schema currently defines only function.
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub type_: Option<StreamToolCallType>,
    /// Incremental function payload.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function: Option<StreamToolFunction>,
}

/// Tool kind accepted by the frozen chat-stream schema.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StreamToolCallType {
    /// A function invocation.
    Function,
}

/// Incremental function fields carried by a streaming tool call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamToolFunction {
    /// Function name; omitted after the initial delta.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// JSON argument fragment; omitted when this delta carries other metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunk_accepts_an_omitted_id() {
        let chunk: ChatStreamResponse = serde_json::from_str(r#"{"choices":[]}"#).unwrap();
        assert!(chunk.id.is_none());
    }

    #[test]
    fn incremental_tool_function_fields_may_arrive_separately() {
        let chunk: ChatStreamResponse = serde_json::from_value(serde_json::json!({
            "choices": [{
                "delta": {
                    "tool_calls": [
                        {"index": 0, "type": "function", "function": {"name": "lookup"}},
                        {"index": 0, "function": {"arguments": "{\"city\":"}}
                    ]
                }
            }]
        }))
        .unwrap();
        let calls = chunk.choices[0]
            .delta
            .as_ref()
            .unwrap()
            .tool_calls
            .as_ref()
            .unwrap();
        assert_eq!(
            calls[0].function.as_ref().unwrap().name.as_deref(),
            Some("lookup")
        );
        assert!(calls[1].function.as_ref().unwrap().name.is_none());
    }
}
