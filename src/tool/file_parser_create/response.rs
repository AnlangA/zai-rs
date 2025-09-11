//! File parser creation response models.
//!
//! This module provides data structures for file parser task creation responses.

use serde::{Deserialize, Serialize};

/// Response from file parser task creation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileParserCreateResponse {
    /// Message about the task creation
    pub message: String,
    /// Unique identifier for the parsing task
    pub task_id: String,
}

impl FileParserCreateResponse {
    /// Check if the task was created successfully.
    pub fn is_success(&self) -> bool {
        !self.task_id.is_empty()
    }

    /// Get the task ID.
    pub fn task_id(&self) -> &str {
        &self.task_id
    }
}
