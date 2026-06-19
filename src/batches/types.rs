use serde::{Deserialize, Serialize};
use serde_json::Value;
use validator::Validate;

/// Batch task item shared by multiple endpoints
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct BatchItem {
    /// Batch id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Object kind (e.g. `"batch"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object: Option<String>,
    /// Endpoint the batch targets (e.g. `/v4/chat/completions`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,
    /// Id of the uploaded `.jsonl` input file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_file_id: Option<String>,
    /// Completion window for the batch.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completion_window: Option<String>,
    /// Current status (kept as string for forward compatibility)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    /// Id of the produced output file (on success).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_file_id: Option<String>,
    /// Id of the error file (on partial failure).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_file_id: Option<String>,

    // Timestamps (UNIX seconds)
    /// When the batch was created.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<u64>,
    /// When the batch entered the in-progress state.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub in_progress_at: Option<u64>,
    /// When the batch results expire.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<u64>,
    /// When the batch entered the finalizing state.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finalizing_at: Option<u64>,
    /// When the batch completed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<u64>,
    /// When the batch failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failed_at: Option<u64>,
    /// When the batch expired.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expired_at: Option<u64>,
    /// When a cancel was requested.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cancelling_at: Option<u64>,
    /// When the batch was cancelled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cancelled_at: Option<u64>,

    // Counts
    /// Some servers return an aggregated object here; keep it flexible.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_counts: Option<Value>,
    /// Total request count (when reported flat).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<u64>,
    /// Completed request count (when reported flat).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed: Option<u64>,
    /// Failed request count (when reported flat).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failed: Option<u64>,

    /// Metadata bag
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
}
