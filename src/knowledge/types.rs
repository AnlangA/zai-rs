use serde::{Deserialize, Deserializer, Serialize, de::Error as _};
use validator::Validate;

/// Standard knowledge API success envelope.
///
/// The upstream OpenAPI marks every top-level field optional, but the frozen
/// knowledge contract requires a non-null `data` payload and business code
/// `200`. Deserialization enforces that invariant so a partial HTTP-200 body
/// cannot be mistaken for success. Public fields remain optional for source
/// compatibility and caller-constructed/serialized values.
#[derive(Debug, Clone, Serialize, Validate)]
pub struct KnowledgeResponse<T> {
    /// Endpoint-specific response payload. Successful deserialization requires
    /// this field to be present and non-null.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    /// Business status code. Successful deserialization requires exactly
    /// `200`; the shared transport also rejects explicit business failures.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<i64>,
    /// Human-readable status message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// Server timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<u64>,
}

#[derive(Deserialize)]
struct KnowledgeResponseWire<T> {
    data: Option<T>,
    code: Option<i64>,
    message: Option<String>,
    timestamp: Option<u64>,
}

impl<'de, T> Deserialize<'de> for KnowledgeResponse<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = KnowledgeResponseWire::deserialize(deserializer)?;
        if wire.code != Some(200) {
            return Err(D::Error::custom(
                "knowledge response must contain business code 200",
            ));
        }
        if wire.data.is_none() {
            return Err(D::Error::custom(
                "knowledge response must contain non-null data",
            ));
        }
        Ok(Self {
            data: wire.data,
            code: wire.code,
            message: wire.message,
            timestamp: wire.timestamp,
        })
    }
}

/// Standard knowledge API response envelope for operations without a payload.
#[derive(Debug, Clone, Serialize, Validate)]
pub struct KnowledgeOperationResponse {
    /// Business status code. Successful deserialization requires exactly
    /// `200`; the shared transport also rejects explicit business failures.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<i64>,
    /// Human-readable status message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// Server timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<u64>,
}

#[derive(Deserialize)]
struct KnowledgeOperationResponseWire {
    code: Option<i64>,
    message: Option<String>,
    timestamp: Option<u64>,
}

impl<'de> Deserialize<'de> for KnowledgeOperationResponse {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = KnowledgeOperationResponseWire::deserialize(deserializer)?;
        if wire.code != Some(200) {
            return Err(D::Error::custom(
                "knowledge operation response must contain business code 200",
            ));
        }
        Ok(Self {
            code: wire.code,
            message: wire.message,
            timestamp: wire.timestamp,
        })
    }
}

/// Knowledge base item
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct KnowledgeItem {
    /// Knowledge base id
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Embedding model id
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding_id: Option<u64>,
    /// Whether contextual retrieval is enabled (`0` or `1`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contextual: Option<u8>,
    /// Knowledge base name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Knowledge base description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Background color
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background: Option<String>,
    /// Icon URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    /// Number of documents
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document_size: Option<u64>,
    /// Total tokenized length
    #[serde(skip_serializing_if = "Option::is_none")]
    pub length: Option<u64>,
    /// Total words
    #[serde(skip_serializing_if = "Option::is_none")]
    pub word_num: Option<u64>,
}

/// Knowledge list data payload
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct KnowledgeListData {
    /// Knowledge list
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list: Option<Vec<KnowledgeItem>>,
    /// Total count
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<u64>,
}

/// Knowledge list response envelope.
pub type KnowledgeListResponse = KnowledgeResponse<KnowledgeListData>;

/// Knowledge detail response envelope (data is a single item).
pub type KnowledgeGetResponse = KnowledgeResponse<KnowledgeItem>;

/// Capacity usage counters
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct KnowledgeUsageCounts {
    /// Total words
    #[serde(skip_serializing_if = "Option::is_none")]
    pub word_num: Option<u64>,
    /// Total bytes (length)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub length: Option<u64>,
}

/// Capacity data payload
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct KnowledgeCapacityData {
    /// Used usage
    #[serde(skip_serializing_if = "Option::is_none")]
    pub used: Option<KnowledgeUsageCounts>,
    /// Total quota
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<KnowledgeUsageCounts>,
}

/// Capacity response envelope.
pub type KnowledgeCapacityResponse = KnowledgeResponse<KnowledgeCapacityData>;

/// Document vectorization failure info
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct DocumentFailInfo {
    /// Embedding failure code
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding_code: Option<i64>,
    /// Embedding failure message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding_msg: Option<String>,
}

/// Document item in a knowledge base
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct DocumentItem {
    /// Document id
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Slice type (integer)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub knowledge_type: Option<i64>,
    /// Custom separators
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_separator: Option<Vec<String>>,
    /// Sentence size (slice size)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sentence_size: Option<u64>,
    /// Document length (bytes)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub length: Option<u64>,
    /// Document words
    #[serde(skip_serializing_if = "Option::is_none")]
    pub word_num: Option<u64>,
    /// Document name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Document URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Embedding status (integer)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding_stat: Option<i64>,
    /// Failure info (camelCase in API)
    #[serde(rename = "failInfo", skip_serializing_if = "Option::is_none")]
    pub fail_info: Option<DocumentFailInfo>,
}

/// Document detail response envelope (data is a single document item).
pub type DocumentGetResponse = KnowledgeResponse<DocumentItem>;

/// Inner data of [`DocumentListResponse`] — the document list and total count.
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct DocumentListData {
    /// Documents list
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list: Option<Vec<DocumentItem>>,
    /// Total count
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<u64>,
}

/// Document list response envelope.
pub type DocumentListResponse = KnowledgeResponse<DocumentListData>;

/// Success info for URL upload
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct DocumentUrlUploadSuccessInfo {
    /// Created document id
    #[serde(rename = "documentId", skip_serializing_if = "Option::is_none")]
    pub document_id: Option<String>,
    /// Source URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

/// Failed info for URL upload
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct DocumentUrlUploadFailedInfo {
    /// Source URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Failure reason
    #[serde(rename = "failReason", skip_serializing_if = "Option::is_none")]
    pub fail_reason: Option<String>,
}

/// Upload URL response data payload
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct DocumentUrlUploadData {
    /// Success items
    #[serde(rename = "successInfos", skip_serializing_if = "Option::is_none")]
    pub success_infos: Option<Vec<DocumentUrlUploadSuccessInfo>>,
    /// Failed items
    #[serde(rename = "failedInfos", skip_serializing_if = "Option::is_none")]
    pub failed_infos: Option<Vec<DocumentUrlUploadFailedInfo>>,
}

/// Upload-by-URL response envelope.
pub type DocumentUrlUploadResponse = KnowledgeResponse<DocumentUrlUploadData>;

/// Success info for file upload
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct DocumentUploadSuccessInfo {
    /// Created document id
    #[serde(rename = "documentId", skip_serializing_if = "Option::is_none")]
    pub document_id: Option<String>,
    /// Original file name
    #[serde(rename = "fileName", skip_serializing_if = "Option::is_none")]
    pub file_name: Option<String>,
}

/// Failed info for file upload
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct DocumentUploadFailedInfo {
    /// Original file name
    #[serde(rename = "fileName", skip_serializing_if = "Option::is_none")]
    pub file_name: Option<String>,
    /// Failure reason
    #[serde(rename = "failReason", skip_serializing_if = "Option::is_none")]
    pub fail_reason: Option<String>,
}

/// Upload file response data payload
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct DocumentUploadData {
    /// Success items
    #[serde(rename = "successInfos", skip_serializing_if = "Option::is_none")]
    pub success_infos: Option<Vec<DocumentUploadSuccessInfo>>,
    /// Failed items
    #[serde(rename = "failedInfos", skip_serializing_if = "Option::is_none")]
    pub failed_infos: Option<Vec<DocumentUploadFailedInfo>>,
}

/// One parsed image mapping item
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct DocumentImageItem {
    /// Provider-generated image index marker.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// Image URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cos_url: Option<String>,
}

/// Image list data payload
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct DocumentImageListData {
    /// Images array
    #[serde(skip_serializing_if = "Option::is_none")]
    pub images: Option<Vec<DocumentImageItem>>,
}

/// Parsed-image list response envelope.
pub type DocumentImageListResponse = KnowledgeResponse<DocumentImageListData>;

/// File-upload response envelope.
pub type DocumentUploadResponse = KnowledgeResponse<DocumentUploadData>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_envelopes_require_code_200_and_non_null_data() {
        let valid = serde_json::from_value::<KnowledgeResponse<serde_json::Value>>(
            serde_json::json!({"code": 200, "data": {}}),
        )
        .unwrap();
        assert_eq!(valid.code, Some(200));
        assert_eq!(valid.data, Some(serde_json::json!({})));

        for invalid in [
            serde_json::json!({"code": 200}),
            serde_json::json!({"data": {}}),
            serde_json::json!({"code": 0, "data": {}}),
            serde_json::json!({"code": 201, "data": {}}),
            serde_json::json!({"code": 200, "data": null}),
            serde_json::json!({}),
            serde_json::json!({"data": null, "code": null}),
        ] {
            assert!(
                serde_json::from_value::<KnowledgeResponse<serde_json::Value>>(invalid).is_err()
            );
        }
    }

    #[test]
    fn operation_envelopes_require_code_200_without_inventing_data() {
        assert!(
            serde_json::from_value::<KnowledgeOperationResponse>(serde_json::json!({
                "unknown": "value"
            }))
            .is_err()
        );
        assert!(
            serde_json::from_value::<KnowledgeOperationResponse>(serde_json::json!({
                "message": "ok"
            }))
            .is_err()
        );
        assert!(
            serde_json::from_value::<KnowledgeOperationResponse>(serde_json::json!({
                "code": 200,
                "message": "ok"
            }))
            .is_ok()
        );
        assert!(
            serde_json::from_value::<KnowledgeOperationResponse>(serde_json::json!({
                "code": 0,
                "message": "ok"
            }))
            .is_err()
        );
    }
}
