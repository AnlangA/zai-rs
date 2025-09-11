//! File parser result request models and types.
//!
//! This module provides data structures for file parser result requests,
//! supporting multiple result formats.

use serde::{Deserialize, Serialize};

/// Result format types for parser output.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FormatType {
    /// Return result as plain text
    Text,
    /// Return result as download link
    DownloadLink,
}

impl std::fmt::Display for FormatType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FormatType::Text => write!(f, "text"),
            FormatType::DownloadLink => write!(f, "download_link"),
        }
    }
}
