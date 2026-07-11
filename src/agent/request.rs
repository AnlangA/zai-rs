//! Agent v1 request value types (plan §13.2).
//!
//! The request *builders* and the typed structs live in [`crate::agent`]; this
//! module holds the shared value types (`AgentMessage`, `AgentContent`).

use serde::{Deserialize, Serialize};

/// A single agent message. `role` is validated to be system/user/assistant at
/// build time; `content` follows the official string | object | array union.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMessage {
    /// `system`, `user`, or `assistant`.
    pub role: String,
    /// The message content (string, object, or array of content parts).
    pub content: AgentContent,
}

/// Agent message content: a string, a single content-part object, or an array
/// of content parts (matching the official union). Stored as a JSON value so the
/// SDK never drops fields the contract allows.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AgentContent {
    /// Plain text content.
    Text(String),
    /// A structured JSON value (object or array).
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
