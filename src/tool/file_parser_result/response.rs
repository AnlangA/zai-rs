//! File-parser result response types.

use serde::{Deserialize, Serialize};

use super::request::FormatType;

/// Task processing status.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ParserStatus {
    /// The task is still being processed.
    Processing,
    /// The task completed successfully.
    Succeeded,
    /// The task failed.
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

/// Current state and available output for a parsing task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileParseResultResponse {
    /// Current processing status.
    pub status: ParserStatus,
    /// Provider status message.
    pub message: String,
    /// Unique parsing-task identifier.
    pub task_id: String,
    /// Parsed text when the requested format is [`FormatType::Text`].
    pub content: Option<String>,
    /// Download URL when the requested format is [`FormatType::DownloadLink`].
    #[serde(rename = "parsing_result_url")]
    pub parsing_result_url: Option<String>,
}

impl FileParseResultResponse {
    /// Return whether the task completed successfully.
    pub fn is_success(&self) -> bool {
        self.status == ParserStatus::Succeeded
    }

    /// Return whether the task is still processing.
    pub fn is_processing(&self) -> bool {
        self.status == ParserStatus::Processing
    }

    /// Return whether the task failed.
    pub fn is_failed(&self) -> bool {
        self.status == ParserStatus::Failed
    }

    /// Borrow the task identifier.
    pub fn task_id(&self) -> &str {
        &self.task_id
    }

    /// Borrow parsed text, when available.
    pub fn content(&self) -> Option<&str> {
        self.content.as_deref()
    }

    /// Borrow the result download URL, when available.
    pub fn download_url(&self) -> Option<&str> {
        self.parsing_result_url.as_deref()
    }

    /// Borrow the value corresponding to `format_type`, when available.
    pub fn get_result(&self, format_type: &FormatType) -> Option<&str> {
        match format_type {
            FormatType::Text => self.content(),
            FormatType::DownloadLink => self.download_url(),
        }
    }
}
