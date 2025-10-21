//! # Streaming Response Types for Chat API Models
//!
//! This module defines the data structures used for processing streaming responses
//! from chat completion APIs. These types are specifically designed to handle
//! Server-Sent Events (SSE) data chunks where responses arrive incrementally.
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
//! ## Usage
//!
//! ```rust,ignore
//! let mut client = ChatCompletion::new(model, messages, api_key).enable_stream();
//! client.stream_for_each(|chunk| async move {
//!     if let Some(delta) = &chunk.choices[0].delta {
//!         if let Some(content) = &delta.content {
//!             print!("{}", content);
//!         }
//!     }
//!     Ok(())
//! }).await?;
//! ```

use serde::{Deserialize, Deserializer, Serialize};

/// Custom deserializer that accepts strings or numbers, converting to Option<String>.
///
/// This helper function handles the wire format flexibility where IDs may be
/// transmitted as either strings or numbers, normalizing them to Option<String>.
///
/// ## Supported Formats
///
/// - `null` → `None`
/// - `"string_id"` → `Some("string_id")`
/// - `123` → `Some("123")`
/// - Other types → deserialization error
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
    /// May be a string or number in the wire format, converted to `Option<String>`.
    #[serde(
        skip_serializing_if = "Option::is_none",
        deserialize_with = "de_opt_string_from_number_or_string"
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
    /// When `tool_stream` is enabled and the model emits tool calling information,
    /// providers often stream this as an array of objects with partial fields.
    /// Use a flexible Value here to accept strings/arrays/objects without failing
    /// deserialization on type mismatch across increments.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<crate::model::chat_base_response::ToolCallMessage>>,
}
