use serde::{Deserialize, Deserializer, Serialize, de::Error as _};
use validator::Validate;

/// Response for files listing
#[derive(Debug, Clone, Serialize, Validate)]
pub struct FileListResponse {
    /// Response type: expected "list". An unknown string marker maps to `None`
    /// while the remaining documented payload is preserved.
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
    #[serde(default, deserialize_with = "deserialize_optional_file_list_object")]
    object: Option<FileListObject>,
    data: Option<Vec<FileObject>>,
    has_more: Option<bool>,
}

enum FileListObjectWire {
    List,
    Unknown,
}

impl<'de> Deserialize<'de> for FileListObjectWire {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct MarkerVisitor;

        impl serde::de::Visitor<'_> for MarkerVisitor {
            type Value = FileListObjectWire;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("a file-list object marker string")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(match value {
                    "list" => FileListObjectWire::List,
                    _ => FileListObjectWire::Unknown,
                })
            }
        }

        deserializer.deserialize_str(MarkerVisitor)
    }
}

fn deserialize_optional_file_list_object<'de, D>(
    deserializer: D,
) -> Result<Option<FileListObject>, D::Error>
where
    D: Deserializer<'de>,
{
    let marker = Option::<FileListObjectWire>::deserialize(deserializer)?;
    Ok(match marker {
        Some(FileListObjectWire::List) => Some(FileListObject::List),
        Some(FileListObjectWire::Unknown) | None => None,
    })
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
    /// Object type: expected "file". An unknown string marker maps to `None`
    /// while the remaining documented payload is preserved.
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
    #[serde(default, deserialize_with = "deserialize_optional_file_object_kind")]
    object: Option<FileObjectKind>,
    bytes: Option<u64>,
    created_at: Option<u64>,
    filename: Option<String>,
    purpose: Option<String>,
}

enum FileObjectKindWire {
    File,
    Unknown,
}

impl<'de> Deserialize<'de> for FileObjectKindWire {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct MarkerVisitor;

        impl serde::de::Visitor<'_> for MarkerVisitor {
            type Value = FileObjectKindWire;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("a file object marker string")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(match value {
                    "file" => FileObjectKindWire::File,
                    _ => FileObjectKindWire::Unknown,
                })
            }
        }

        deserializer.deserialize_str(MarkerVisitor)
    }
}

fn deserialize_optional_file_object_kind<'de, D>(
    deserializer: D,
) -> Result<Option<FileObjectKind>, D::Error>
where
    D: Deserializer<'de>,
{
    let marker = Option::<FileObjectKindWire>::deserialize(deserializer)?;
    Ok(match marker {
        Some(FileObjectKindWire::File) => Some(FileObjectKind::File),
        Some(FileObjectKindWire::Unknown) | None => None,
    })
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
    /// Resource type: expected "file". An unknown string marker maps to `None`
    /// while the remaining documented payload is preserved.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object: Option<FileObjectKind>,
    /// Whether deletion succeeded
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deleted: Option<bool>,
}

#[derive(Deserialize)]
struct FileDeleteResponseWire {
    id: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_file_object_kind")]
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
        assert!(serde_json::from_str::<FileListResponse>(r#"{"object":"future"}"#).is_err());
        assert!(
            serde_json::from_str::<FileListResponse>(
                r#"{"object":"future","data":null,"has_more":null}"#
            )
            .is_err()
        );
        assert!(serde_json::from_str::<FileObject>("{}").is_err());
        assert!(serde_json::from_str::<FileObject>(r#"{"bytes":0}"#).is_ok());
        assert!(serde_json::from_str::<FileObject>(r#"{"object":"future"}"#).is_err());
        assert!(serde_json::from_str::<FileObject>(r#"{"object":"future","id":null}"#).is_err());
        assert!(serde_json::from_str::<FileDeleteResponse>("{}").is_err());
        assert!(serde_json::from_str::<FileDeleteResponse>(r#"{"deleted":false}"#).is_ok());
        assert!(serde_json::from_str::<FileDeleteResponse>(r#"{"object":"future"}"#).is_err());
        assert!(
            serde_json::from_str::<FileDeleteResponse>(r#"{"object":"future","deleted":null}"#)
                .is_err()
        );
    }

    #[test]
    fn file_list_marker_preserves_payload_and_strict_types() {
        let future: FileListResponse = serde_json::from_str(
            r#"{"object":"future_list","data":[{"id":"nested-file","object":"future_file"}],"has_more":false}"#,
        )
        .unwrap();
        assert!(future.object.is_none());
        let nested = &future.data.as_ref().unwrap()[0];
        assert_eq!(nested.id.as_deref(), Some("nested-file"));
        assert!(nested.object.is_none());
        assert_eq!(future.has_more, Some(false));

        let known: FileListResponse = serde_json::from_str(r#"{"object":"list"}"#).unwrap();
        assert!(matches!(known.object, Some(FileListObject::List)));
        let missing: FileListResponse = serde_json::from_str(r#"{"has_more":true}"#).unwrap();
        assert!(missing.object.is_none());
        let null: FileListResponse = serde_json::from_str(r#"{"object":null,"data":[]}"#).unwrap();
        assert!(null.object.is_none());

        for marker in ["1", "true", "[]", "{}", r#"{"future-list":null}"#] {
            let text = format!(r#"{{"object":{marker},"has_more":false}}"#);
            assert!(serde_json::from_str::<FileListResponse>(&text).is_err());
        }
        assert!(serde_json::from_str::<FileListObject>(r#""future_list""#).is_err());
    }

    #[test]
    fn file_and_delete_markers_preserve_payload_and_strict_types() {
        let future: FileObject = serde_json::from_str(
            r#"{"id":"file-1","object":"future_file","bytes":7,"filename":"a.txt"}"#,
        )
        .unwrap();
        assert!(future.object.is_none());
        assert_eq!(future.id.as_deref(), Some("file-1"));
        assert_eq!(future.bytes, Some(7));
        assert_eq!(future.filename.as_deref(), Some("a.txt"));

        let known: FileObject = serde_json::from_str(r#"{"object":"file"}"#).unwrap();
        assert!(matches!(known.object, Some(FileObjectKind::File)));
        let missing: FileObject = serde_json::from_str(r#"{"id":"file-2"}"#).unwrap();
        assert!(missing.object.is_none());
        let null: FileObject =
            serde_json::from_str(r#"{"object":null,"purpose":"batch"}"#).unwrap();
        assert!(null.object.is_none());

        let deleted: FileDeleteResponse =
            serde_json::from_str(r#"{"id":"file-3","object":"future_file","deleted":false}"#)
                .unwrap();
        assert!(deleted.object.is_none());
        assert_eq!(deleted.id.as_deref(), Some("file-3"));
        assert_eq!(deleted.deleted, Some(false));
        let known_deleted: FileDeleteResponse =
            serde_json::from_str(r#"{"object":"file"}"#).unwrap();
        assert!(matches!(known_deleted.object, Some(FileObjectKind::File)));
        let missing_deleted: FileDeleteResponse =
            serde_json::from_str(r#"{"deleted":true}"#).unwrap();
        assert!(missing_deleted.object.is_none());
        let null_deleted: FileDeleteResponse =
            serde_json::from_str(r#"{"object":null,"id":"file-4"}"#).unwrap();
        assert!(null_deleted.object.is_none());

        for marker in ["1", "true", "[]", "{}", r#"{"future-file":null}"#] {
            let object = format!(r#"{{"object":{marker},"id":"file-5"}}"#);
            assert!(serde_json::from_str::<FileObject>(&object).is_err());
            let delete = format!(r#"{{"object":{marker},"deleted":true}}"#);
            assert!(serde_json::from_str::<FileDeleteResponse>(&delete).is_err());
        }
        assert!(serde_json::from_str::<FileObjectKind>(r#""future_file""#).is_err());

        let encoded_list = serde_json::to_value(FileListResponse {
            object: Some(FileListObject::List),
            data: None,
            has_more: None,
        })
        .unwrap();
        let encoded_object = serde_json::to_value(FileObject {
            id: None,
            object: Some(FileObjectKind::File),
            bytes: None,
            created_at: None,
            filename: None,
            purpose: None,
        })
        .unwrap();
        let encoded_delete = serde_json::to_value(FileDeleteResponse {
            id: None,
            object: Some(FileObjectKind::File),
            deleted: None,
        })
        .unwrap();
        assert_eq!(encoded_list["object"], "list");
        assert_eq!(encoded_object["object"], "file");
        assert_eq!(encoded_delete["object"], "file");
    }
}
