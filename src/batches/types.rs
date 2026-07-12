use std::collections::BTreeMap;

use serde::{Deserialize, Deserializer, Serialize, de::Error as _};
use validator::Validate;

/// Batch task item shared by multiple endpoints
#[derive(Debug, Clone, Serialize, Validate)]
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
    /// Batch request count returned by the service.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_counts: Option<u64>,
    /// Total request count (when reported flat).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<u64>,
    /// Completed request count (when reported flat).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed: Option<u64>,
    /// Failed request count (when reported flat).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failed: Option<u64>,

    /// String metadata attached to the batch.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<BTreeMap<String, String>>,
}

#[derive(Deserialize)]
struct BatchItemWire {
    id: Option<String>,
    object: Option<String>,
    endpoint: Option<String>,
    input_file_id: Option<String>,
    completion_window: Option<String>,
    status: Option<String>,
    output_file_id: Option<String>,
    error_file_id: Option<String>,
    created_at: Option<u64>,
    in_progress_at: Option<u64>,
    expires_at: Option<u64>,
    finalizing_at: Option<u64>,
    completed_at: Option<u64>,
    failed_at: Option<u64>,
    expired_at: Option<u64>,
    cancelling_at: Option<u64>,
    cancelled_at: Option<u64>,
    request_counts: Option<u64>,
    total: Option<u64>,
    completed: Option<u64>,
    failed: Option<u64>,
    metadata: Option<BTreeMap<String, String>>,
}

impl<'de> Deserialize<'de> for BatchItem {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = BatchItemWire::deserialize(deserializer)?;
        let has_documented_field = wire.id.is_some()
            || wire.object.is_some()
            || wire.endpoint.is_some()
            || wire.input_file_id.is_some()
            || wire.completion_window.is_some()
            || wire.status.is_some()
            || wire.output_file_id.is_some()
            || wire.error_file_id.is_some()
            || wire.created_at.is_some()
            || wire.in_progress_at.is_some()
            || wire.expires_at.is_some()
            || wire.finalizing_at.is_some()
            || wire.completed_at.is_some()
            || wire.failed_at.is_some()
            || wire.expired_at.is_some()
            || wire.cancelling_at.is_some()
            || wire.cancelled_at.is_some()
            || wire.request_counts.is_some()
            || wire.total.is_some()
            || wire.completed.is_some()
            || wire.failed.is_some()
            || wire.metadata.is_some();
        if !has_documented_field {
            return Err(D::Error::custom(
                "batch response contained no documented non-null fields",
            ));
        }
        Ok(Self {
            id: wire.id,
            object: wire.object,
            endpoint: wire.endpoint,
            input_file_id: wire.input_file_id,
            completion_window: wire.completion_window,
            status: wire.status,
            output_file_id: wire.output_file_id,
            error_file_id: wire.error_file_id,
            created_at: wire.created_at,
            in_progress_at: wire.in_progress_at,
            expires_at: wire.expires_at,
            finalizing_at: wire.finalizing_at,
            completed_at: wire.completed_at,
            failed_at: wire.failed_at,
            expired_at: wire.expired_at,
            cancelling_at: wire.cancelling_at,
            cancelled_at: wire.cancelled_at,
            request_counts: wire.request_counts,
            total: wire.total,
            completed: wire.completed,
            failed: wire.failed,
            metadata: wire.metadata,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn batch_response_requires_a_documented_non_null_field() {
        assert!(serde_json::from_str::<BatchItem>("{}").is_err());
        assert!(serde_json::from_str::<BatchItem>(r#"{"id":null}"#).is_err());
        assert!(serde_json::from_str::<BatchItem>(r#"{"request_counts":0}"#).is_ok());
        assert!(serde_json::from_str::<BatchItem>(r#"{"metadata":{}}"#).is_ok());
        assert!(serde_json::from_str::<BatchItem>(r#"{"request_counts":{}}"#).is_err());
    }
}
