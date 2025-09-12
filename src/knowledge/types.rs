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
