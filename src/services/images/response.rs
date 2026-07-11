//! Images response types (plan P06).
//!
//! All fields use `#[serde(default)]` for forward-compatibility.

use serde::Deserialize;

/// Response from an async image generation request.
#[derive(Debug, Clone, Deserialize)]
pub struct AsyncImageGenerationResponse {
    /// Task identifier for polling status.
    #[serde(default)]
    pub task_id: Option<String>,
    /// Task status (e.g. "PROCESSING", "SUCCESS").
    #[serde(default)]
    pub task_status: Option<String>,
    /// Additional response data (open schema).
    #[serde(default)]
    pub data: serde_json::Value,
}
