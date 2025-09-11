//! File parser result response models.
//!
//! This module provides data structures for file parser result responses,
//! including task status and parsed content.

use super::request::FormatType;
use serde::{Deserialize, Serialize};

/// Task processing status.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ParserStatus {
    /// Task is currently being processed
    Processing,
    /// Task completed successfully
    Succeeded,
    /// Task failed to complete
    Failed,
}

impl std::fmt::Display for ParserStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParserStatus::Processing => write!(f, "processing"),
            ParserStatus::Succeeded => write!(f, "succeeded"),
            ParserStatus::Failed => write!(f, "failed"),
        }
    }
}

/// Response from file parser result retrieval.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileParserResultResponse {
    /// Current processing status of the task
    pub status: ParserStatus,
    /// Message about the result status
    pub message: String,
    /// Unique identifier for the parsing task
    pub task_id: String,
    /// Parsed text content (when format_type=text)
    pub content: Option<String>,
    /// Download link for results (when format_type=download_link)
    #[serde(rename = "parsing_result_url")]
    pub parsing_result_url: Option<String>,
}

impl FileParserResultResponse {
    /// Check if the task completed successfully.
    pub fn is_success(&self) -> bool {
        self.status == ParserStatus::Succeeded
    }

    /// Check if the task is still processing.
    pub fn is_processing(&self) -> bool {
        self.status == ParserStatus::Processing
    }

    /// Check if the task failed.
    pub fn is_failed(&self) -> bool {
        self.status == ParserStatus::Failed
    }

    /// Get the task ID.
    pub fn task_id(&self) -> &str {
        &self.task_id
    }

    /// Get the parsed content if available.
    pub fn content(&self) -> Option<&str> {
        self.content.as_deref()
    }

    /// Get the download URL if available.
    pub fn download_url(&self) -> Option<&str> {
        self.parsing_result_url.as_deref()
    }

    /// Get the result based on the format type.
    pub fn get_result(&self, format_type: &FormatType) -> Option<&str> {
        match format_type {
            FormatType::Text => self.content(),
            FormatType::DownloadLink => self.download_url(),
        }
    }
}
