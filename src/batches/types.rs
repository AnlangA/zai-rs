use serde::{Deserialize, Serialize};
use serde_json::Value;
use validator::Validate;

/// Batch task item shared by multiple endpoints
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct BatchItem {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_file_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completion_window: Option<String>,
    /// Current status (kept as string for forward compatibility)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_file_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_file_id: Option<String>,

    // Timestamps (UNIX seconds)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub in_progress_at: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finalizing_at: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failed_at: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expired_at: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cancelling_at: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cancelled_at: Option<u64>,

    // Counts
    /// Some servers return an aggregated object here; keep it flexible.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_counts: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failed: Option<u64>,

    /// Metadata bag
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
}
