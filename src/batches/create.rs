use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::{ZaiResult, client::ZaiClient};

/// Endpoint for batch requests
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BatchEndpoint {
    /// Chat completions endpoint
    #[serde(rename = "/v4/chat/completions")]
    ChatCompletions,
}

/// Request body for creating a batch task
#[derive(Clone, Serialize, Deserialize, Validate)]
pub struct BatchCreateBody {
    /// ID of the uploaded .jsonl file (purpose must be "batch")
    #[validate(length(min = 1))]
    pub input_file_id: String,

    /// Endpoint to be used for all requests in the batch
    pub endpoint: BatchEndpoint,

    /// Whether to auto delete input file after processing (default: true)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_delete_input_file: Option<bool>,

    /// String metadata for task management and tracking (up to 16 key-value pairs).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<BTreeMap<String, String>>,
}

impl std::fmt::Debug for BatchCreateBody {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("BatchCreateBody")
            .field("input_file_id", &"[REDACTED]")
            .field("endpoint", &self.endpoint)
            .field("auto_delete_input_file", &self.auto_delete_input_file)
            .field(
                "metadata_entries",
                &self.metadata.as_ref().map(BTreeMap::len),
            )
            .finish()
    }
}

impl BatchCreateBody {
    /// Create a new batch body from an input file id and target endpoint
    /// (auto-delete defaults to `true`).
    pub fn new(input_file_id: impl Into<String>, endpoint: BatchEndpoint) -> Self {
        Self {
            input_file_id: input_file_id.into(),
            endpoint,
            auto_delete_input_file: Some(true),
            metadata: None,
        }
    }

    /// Set auto delete flag
    pub fn with_auto_delete_input_file(mut self, v: bool) -> Self {
        self.auto_delete_input_file = Some(v);
        self
    }

    /// Set string metadata.
    pub fn with_metadata(mut self, v: BTreeMap<String, String>) -> Self {
        self.metadata = Some(v);
        self
    }
}

/// Create batch request (POST /paas/v4/batches)
pub struct BatchCreateRequest {
    /// Request body.
    body: BatchCreateBody,
}

impl BatchCreateRequest {
    /// Build a new create-batch request with required fields
    pub fn new(input_file_id: impl Into<String>, endpoint: BatchEndpoint) -> Self {
        let body = BatchCreateBody::new(input_file_id, endpoint);
        Self { body }
    }

    /// Set auto-delete flag (default true)
    pub fn with_auto_delete_input_file(mut self, v: bool) -> Self {
        self.body = self.body.with_auto_delete_input_file(v);
        self
    }

    /// Set string metadata.
    pub fn with_metadata(mut self, v: BTreeMap<String, String>) -> Self {
        self.body = self.body.with_metadata(v);
        self
    }

    /// Validate body using `validator`
    pub fn validate(&self) -> ZaiResult<()> {
        self.body.validate()?;
        if self.body.input_file_id.trim().is_empty() {
            return Err(crate::client::validation::invalid(
                "input_file_id cannot be blank",
            ));
        }
        if let Some(metadata) = self.body.metadata.as_ref() {
            if metadata.len() > 16 {
                return Err(crate::client::validation::invalid(
                    "metadata cannot contain more than 16 entries",
                ));
            }
            for (key, value) in metadata {
                if key.trim().is_empty() || key.chars().count() > 64 {
                    return Err(crate::client::validation::invalid(
                        "metadata keys must contain between 1 and 64 non-blank characters",
                    ));
                }
                if value.chars().count() > 512 {
                    return Err(crate::client::validation::invalid(
                        "metadata values cannot exceed 512 characters",
                    ));
                }
            }
        }
        Ok(())
    }

    /// Send request via a [`ZaiClient`] and parse typed response
    pub async fn send_via(&self, client: &ZaiClient) -> ZaiResult<BatchCreateResponse> {
        self.validate()?;

        let route = crate::client::routes::BATCHES_CREATE;
        client
            .operation(route)
            .send_json::<_, BatchCreateResponse>(&self.body)
            .await
    }
}

/// Response type for creating a batch task (same as a single item)
pub type BatchCreateResponse = super::types::BatchItem;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metadata_is_a_bounded_string_map() {
        let metadata = BTreeMap::from([("customer".to_owned(), "acme".to_owned())]);
        let request = BatchCreateRequest::new("file-1", BatchEndpoint::ChatCompletions)
            .with_metadata(metadata);
        assert!(request.validate().is_ok());
        assert_eq!(
            serde_json::to_value(&request.body).unwrap()["metadata"]["customer"],
            "acme"
        );

        let too_many = (0..17)
            .map(|index| (format!("key-{index}"), "value".to_owned()))
            .collect();
        assert!(
            BatchCreateRequest::new("file-1", BatchEndpoint::ChatCompletions)
                .with_metadata(too_many)
                .validate()
                .is_err()
        );
        assert!(
            BatchCreateRequest::new("file-1", BatchEndpoint::ChatCompletions)
                .with_metadata(BTreeMap::from([(" ".to_owned(), "value".to_owned())]))
                .validate()
                .is_err()
        );
    }

    #[test]
    fn debug_redacts_file_id_and_metadata() {
        let body =
            BatchCreateBody::new("private-file-id", BatchEndpoint::ChatCompletions).with_metadata(
                BTreeMap::from([("private-key".to_owned(), "private-value".to_owned())]),
            );
        let debug = format!("{body:?}");
        for secret in ["private-file-id", "private-key", "private-value"] {
            assert!(!debug.contains(secret));
        }
        assert!(debug.contains("metadata_entries: Some(1)"));
    }
}
