use serde::{Deserialize, Serialize};
use validator::Validate;

/// Query parameters for listing files.
#[derive(Debug, Clone, Serialize, Validate)]
pub struct FileListQuery {
    /// Pagination cursor
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after: Option<String>,

    /// Filter by file purpose (optional; matches cURL examples which may omit)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub purpose: Option<FilePurpose>,

    /// Sort order (currently only `created_at`)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order: Option<FileOrder>,

    /// Page size 1..=100 (default 20)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(range(min = 1, max = 100))]
    pub limit: Option<u32>,
}

impl Default for FileListQuery {
    fn default() -> Self {
        Self::new()
    }
}

impl FileListQuery {
    /// Create a new empty file-list query.
    pub fn new() -> Self {
        Self {
            after: None,
            purpose: None,
            order: None,
            limit: None,
        }
    }
    /// Set the pagination cursor.
    pub fn with_after(mut self, after: impl Into<String>) -> Self {
        self.after = Some(after.into());
        self
    }
    /// Filter by file purpose.
    pub fn with_purpose(mut self, p: FilePurpose) -> Self {
        self.purpose = Some(p);
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

/// Categorized purpose a file is uploaded for.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FilePurpose {
    /// File used as batch-processing input.
    #[serde(rename = "batch")]
    Batch,
    /// File used for file-extract / parsing.
    #[serde(rename = "file-extract")]
    FileExtract,
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

impl FilePurpose {
    /// Return the canonical upstream string for this purpose.
    pub fn as_str(&self) -> &'static str {
        match self {
            FilePurpose::Batch => "batch",
            FilePurpose::FileExtract => "file-extract",
            FilePurpose::CodeInterpreter => "code-interpreter",
            FilePurpose::Agent => "agent",
            FilePurpose::VoiceCloneInput => "voice-clone-input",
        }
    }
}

/// Sort order for file listing.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
