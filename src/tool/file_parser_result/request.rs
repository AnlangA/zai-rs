//! File-parser result format.

use serde::{Deserialize, Serialize};

/// Representation requested from the parser-result endpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FormatType {
    /// Return parsed plain text.
    Text,
    /// Return a result download URL.
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
