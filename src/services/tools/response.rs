//! Response types for document layout parsing and content extraction.
//!
//! All fields use `#[serde(default)]` for forward-compatibility.

use serde::Deserialize;

/// Response from a layout parsing request.
#[derive(Debug, Clone, Deserialize)]
pub struct LayoutParsingResponse {
    /// Parsed layout result (open schema).
    #[serde(default)]
    pub data: serde_json::Value,
}

/// Response from a reader request.
#[derive(Debug, Clone, Deserialize)]
pub struct ReaderResponse {
    /// Extracted content (open schema).
    #[serde(default)]
    pub data: serde_json::Value,
}
