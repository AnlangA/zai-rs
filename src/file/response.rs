use serde::{Deserialize, Deserializer, Serialize, de::Error as _};
use validator::Validate;

/// Response for files listing
#[derive(Debug, Clone, Serialize, Validate)]
pub struct FileListResponse {
    /// Response type: expected "list"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object: Option<FileListObject>,
    /// File entries
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Vec<FileObject>>,
    /// Whether there are more results
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_more: Option<bool>,
}

/// Object marker returned by the file-list endpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FileListObject {
    /// A list response.
    List,
}

#[derive(Deserialize)]
struct FileListResponseWire {
    object: Option<FileListObject>,
    data: Option<Vec<FileObject>>,
    has_more: Option<bool>,
}

impl<'de> Deserialize<'de> for FileListResponse {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = FileListResponseWire::deserialize(deserializer)?;
        if wire.object.is_none() && wire.data.is_none() && wire.has_more.is_none() {
            return Err(D::Error::custom(
                "file-list response contained no documented non-null fields",
            ));
        }
        Ok(Self {
            object: wire.object,
            data: wire.data,
            has_more: wire.has_more,
        })
    }
}

/// File metadata object (as returned by list/upload APIs)
#[derive(Debug, Clone, Serialize, Validate)]
pub struct FileObject {
    /// Unique file identifier
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Object type: expected "file"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object: Option<FileObjectKind>,
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

/// Response returned by the file-upload operation.
pub type FileUploadResponse = FileObject;

/// Object marker returned for one file resource.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FileObjectKind {
    /// A file resource.
    File,
}

#[derive(Deserialize)]
struct FileObjectWire {
    id: Option<String>,
    object: Option<FileObjectKind>,
    bytes: Option<u64>,
    created_at: Option<u64>,
    filename: Option<String>,
    purpose: Option<String>,
}

impl<'de> Deserialize<'de> for FileObject {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = FileObjectWire::deserialize(deserializer)?;
        if wire.id.is_none()
            && wire.object.is_none()
            && wire.bytes.is_none()
            && wire.created_at.is_none()
            && wire.filename.is_none()
            && wire.purpose.is_none()
        {
            return Err(D::Error::custom(
                "file response contained no documented non-null fields",
            ));
        }
        Ok(Self {
            id: wire.id,
            object: wire.object,
            bytes: wire.bytes,
            created_at: wire.created_at,
            filename: wire.filename,
            purpose: wire.purpose,
        })
    }
}

/// Response for file deletion (DELETE /files/{file_id})
#[derive(Debug, Clone, Serialize, Validate)]
pub struct FileDeleteResponse {
    /// Deleted resource id
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Resource type: expected "file"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object: Option<FileObjectKind>,
    /// Whether deletion succeeded
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deleted: Option<bool>,
}

#[derive(Deserialize)]
struct FileDeleteResponseWire {
    id: Option<String>,
    object: Option<FileObjectKind>,
    deleted: Option<bool>,
}

impl<'de> Deserialize<'de> for FileDeleteResponse {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = FileDeleteResponseWire::deserialize(deserializer)?;
        if wire.id.is_none() && wire.object.is_none() && wire.deleted.is_none() {
            return Err(D::Error::custom(
                "file-delete response contained no documented non-null fields",
            ));
        }
        Ok(Self {
            id: wire.id,
            object: wire.object,
            deleted: wire.deleted,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_responses_reject_empty_success_objects() {
        assert!(serde_json::from_str::<FileListResponse>("{}").is_err());
        assert!(serde_json::from_str::<FileListResponse>(r#"{"data":[]}"#).is_ok());
        assert!(serde_json::from_str::<FileObject>("{}").is_err());
        assert!(serde_json::from_str::<FileObject>(r#"{"bytes":0}"#).is_ok());
        assert!(serde_json::from_str::<FileDeleteResponse>("{}").is_err());
        assert!(serde_json::from_str::<FileDeleteResponse>(r#"{"deleted":false}"#).is_ok());
    }

    #[test]
    fn file_object_markers_are_closed_enums() {
        assert!(serde_json::from_str::<FileListResponse>(r#"{"object":"list"}"#).is_ok());
        assert!(serde_json::from_str::<FileListResponse>(r#"{"object":"future"}"#).is_err());
        assert!(serde_json::from_str::<FileObject>(r#"{"object":"file"}"#).is_ok());
        assert!(serde_json::from_str::<FileObject>(r#"{"object":"future"}"#).is_err());
    }
}
