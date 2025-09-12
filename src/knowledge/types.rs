use serde::{Deserialize, Serialize};
use validator::Validate;

/// Knowledge base item
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct KnowledgeItem {
    /// Knowledge base id
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Embedding model id
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding_id: Option<u64>,
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

/// Knowledge list response envelope
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct KnowledgeListResponse {
    /// Data payload
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<KnowledgeListData>,
    /// Response code (200 means success)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<i64>,
    /// Response message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// Response timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<u64>,
}



/// Knowledge detail response envelope (data is a single item)
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct KnowledgeDetailResponse {
    /// Data payload (single knowledge item)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<KnowledgeItem>,
    /// Response code (200 means success)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<i64>,
    /// Response message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// Response timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<u64>,
}


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

/// Capacity response envelope
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct KnowledgeCapacityResponse {
    /// Data payload (used and total)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<KnowledgeCapacityData>,
    /// Response code (200 means success)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<i64>,
    /// Response message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// Response timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<u64>,
}


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
    #[serde(rename = "failInfo")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fail_info: Option<DocumentFailInfo>,
}

/// Document list data payload

/// Document detail response envelope (data is a single document item)
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct DocumentDetailResponse {
    /// Data payload (single document item)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<DocumentItem>,
    /// Response code
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<i64>,
    /// Response message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// Response timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct DocumentListData {
    /// Documents list
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list: Option<Vec<DocumentItem>>,
    /// Total count
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<u64>,
}

/// Document list response envelope
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct DocumentListResponse {
    /// Data payload
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<DocumentListData>,
    /// Response code
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<i64>,
    /// Response message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// Response timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<u64>,
}


/// Success info for URL upload
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct UploadUrlSuccessInfo {
    /// Created document id
    #[serde(rename = "documentId")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document_id: Option<String>,
    /// Source URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

/// Failed info for URL upload
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct UploadUrlFailedInfo {
    /// Source URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Failure reason
    #[serde(rename = "failReason")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fail_reason: Option<String>,
}

/// Upload URL response data payload
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct UploadUrlData {
    /// Success items
    #[serde(rename = "successInfos")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub success_infos: Option<Vec<UploadUrlSuccessInfo>>,
    /// Failed items
    #[serde(rename = "failedInfos")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failed_infos: Option<Vec<UploadUrlFailedInfo>>,
}

/// Upload URL response envelope
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct UploadUrlResponse {
    /// Data payload
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<UploadUrlData>,
    /// Response code
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<i64>,
    /// Response message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// Response timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<u64>,
}


/// Success info for file upload
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct UploadFileSuccessInfo {
    /// Created document id
    #[serde(rename = "documentId")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document_id: Option<String>,
    /// Original file name
    #[serde(rename = "fileName")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_name: Option<String>,
}

/// Failed info for file upload
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct UploadFileFailedInfo {
    /// Original file name
    #[serde(rename = "fileName")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_name: Option<String>,
    /// Failure reason
    #[serde(rename = "failReason")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fail_reason: Option<String>,
}

/// Upload file response data payload
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct UploadFileData {
    /// Success items
    #[serde(rename = "successInfos")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub success_infos: Option<Vec<UploadFileSuccessInfo>>,
    /// Failed items
    #[serde(rename = "failedInfos")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failed_infos: Option<Vec<UploadFileFailedInfo>>,
}

/// One parsed image mapping item
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct DocumentImageItem {
    /// Image index text, e.g. "【示意图序号_...】"
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

/// Image list response envelope
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct DocumentImageListResponse {
    /// Data payload
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<DocumentImageListData>,
    /// Response code
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<i64>,
    /// Response message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// Response timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<u64>,
}


/// Upload file response envelope
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct UploadFileResponse {
    /// Data payload
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<UploadFileData>,
    /// Response code
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<i64>,
    /// Response message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// Response timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<u64>,
}
