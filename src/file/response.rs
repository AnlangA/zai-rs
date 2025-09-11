use serde::{Deserialize, Serialize};
use validator::Validate;

/// Response for files listing
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct FileListResponse {
    /// Response type: expected "list"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object: Option<String>,
    /// File entries
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Vec<FileObject>>,
    /// Whether there are more results
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_more: Option<bool>,
}

/// File metadata object (as returned by list/upload APIs)
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct FileObject {
    /// Unique file identifier
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Object type: expected "file"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object: Option<String>,
    /// File size in bytes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bytes: Option<u64>,
    /// UNIX timestamp of creation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<u64>,
    /// Original filename
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filename: Option<String>,
    /// Purpose string (e.g., batch, file-extract, ...)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub purpose: Option<String>,
}

/// Response for file deletion (DELETE /files/{file_id})
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct FileDeleteResponse {
    /// Deleted resource id
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Resource type: expected "file"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object: Option<String>,
    /// Whether deletion succeeded
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deleted: Option<bool>,
}
