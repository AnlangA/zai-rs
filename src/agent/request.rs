//! Shared Agent v1 request values.
//!
//! The request *builders* and the typed structs live in [`crate::agent`]; this
//! module holds the shared value types (`AgentMessage`, `AgentContent`).

use serde::{Deserialize, Serialize};

/// A single agent message.
///
/// [`AgentInvokeRequestBuilder`](crate::agent::AgentInvokeRequestBuilder)
/// validates `role` as `system`, `user`, or `assistant`. Other construction
/// paths, including a struct literal and the conversation builder, retain the
/// caller-provided role without validation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMessage {
    /// `system`, `user`, or `assistant`.
    pub role: String,
    /// The message content (string, object, or array of content parts).
    pub content: AgentContent,
}

/// Agent message content represented either as text or an arbitrary JSON value.
///
/// The `Json` variant deliberately does not restrict the value to an object or
/// array; callers are responsible for supplying a shape accepted by the Agent
/// API.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AgentContent {
    /// Plain text content.
    Text(String),
    /// An arbitrary JSON value passed through unchanged.
    Json(serde_json::Value),
}

impl From<String> for AgentContent {
    fn from(s: String) -> Self {
        AgentContent::Text(s)
    }
}

impl From<&str> for AgentContent {
    fn from(s: &str) -> Self {
        AgentContent::Text(s.to_string())
    }
}

impl From<serde_json::Value> for AgentContent {
    fn from(v: serde_json::Value) -> Self {
        AgentContent::Json(v)
    }
}
