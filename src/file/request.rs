use serde::{Deserialize, Serialize};
use validator::Validate;

/// Query parameters for listing files.
#[derive(Clone, Serialize, Validate)]
pub struct FileListQuery {
    /// Pagination cursor
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(length(min = 1))]
    pub(crate) after: Option<String>,

    /// Required file-purpose filter.
    pub(crate) purpose: FileListPurpose,

    /// Sort order (currently only `created_at`)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) order: Option<FileOrder>,

    /// Page size 1..=100 (default 20)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(range(min = 1, max = 100))]
    pub(crate) limit: Option<u32>,
}

impl std::fmt::Debug for FileListQuery {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("FileListQuery")
            .field("after", &self.after.as_ref().map(|_| "[REDACTED]"))
            .field("purpose", &self.purpose)
            .field("order", &self.order)
            .field("limit", &self.limit)
            .finish()
    }
}

impl FileListQuery {
    /// Create a query with the required purpose filter.
    pub fn new(purpose: FileListPurpose) -> Self {
        Self {
            after: None,
            purpose,
            order: None,
            limit: None,
        }
    }
    /// Set the pagination cursor.
    pub fn with_after(mut self, after: impl Into<String>) -> Self {
        self.after = Some(after.into());
        self
    }
    /// Set the sort order.
    pub fn with_order(mut self, o: FileOrder) -> Self {
        self.order = Some(o);
        self
    }
    /// Set the page size (1..=100).
    pub fn with_limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }
}

/// Purpose filter accepted by the file-list operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileListPurpose {
    /// File used as batch-processing input.
    #[serde(rename = "batch")]
    Batch,
    /// File used by the code interpreter.
    #[serde(rename = "code-interpreter")]
    CodeInterpreter,
    /// File attached to an agent.
    #[serde(rename = "agent")]
    Agent,
}

impl FileListPurpose {
    /// Return the canonical upstream string for this purpose.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Batch => "batch",
            Self::CodeInterpreter => "code-interpreter",
            Self::Agent => "agent",
        }
    }
}

/// Purpose accepted by the multipart file-upload operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileUploadPurpose {
    /// File used as batch-processing input.
    #[serde(rename = "batch")]
    Batch,
    /// File used by the code interpreter.
    #[serde(rename = "code-interpreter")]
    CodeInterpreter,
    /// File attached to an agent.
    #[serde(rename = "agent")]
    Agent,
    /// Sample audio used as voice-clone input.
    #[serde(rename = "voice-clone-input")]
    VoiceCloneInput,
}

impl FileUploadPurpose {
    /// Return the exact multipart value.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Batch => "batch",
            Self::CodeInterpreter => "code-interpreter",
            Self::Agent => "agent",
            Self::VoiceCloneInput => "voice-clone-input",
        }
    }
}

/// Sort order for file listing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileOrder {
    /// Order by creation time.
    #[serde(rename = "created_at")]
    CreatedAt,
}

impl FileOrder {
    /// Return the canonical upstream string for this order.
    pub fn as_str(&self) -> &'static str {
        match self {
            FileOrder::CreatedAt => "created_at",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_query_debug_redacts_the_file_cursor() {
        let query = FileListQuery::new(FileListPurpose::Batch).with_after("private-file-id");
        assert!(!format!("{query:?}").contains("private-file-id"));
    }

    #[test]
    fn operation_specific_purpose_enums_match_the_frozen_values() {
        assert_eq!(
            [
                FileListPurpose::Batch,
                FileListPurpose::CodeInterpreter,
                FileListPurpose::Agent,
            ]
            .map(|value| serde_json::to_value(value).unwrap()),
            ["batch", "code-interpreter", "agent"]
                .map(|value| serde_json::Value::String(value.to_owned()))
        );
        assert_eq!(
            [
                FileUploadPurpose::Batch,
                FileUploadPurpose::CodeInterpreter,
                FileUploadPurpose::Agent,
                FileUploadPurpose::VoiceCloneInput,
            ]
            .map(|value| serde_json::to_value(value).unwrap()),
            ["batch", "code-interpreter", "agent", "voice-clone-input"]
                .map(|value| serde_json::Value::String(value.to_owned()))
        );
    }
}
