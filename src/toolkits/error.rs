//! Enhanced error handling with better Rust idioms

use thiserror::Error;

/// Result type for tool operations
pub type ToolResult<T> = Result<T, ToolError>;

/// Enhanced error type with better context and error chaining
#[derive(Error, Debug)]
pub enum ToolError {
    #[error("Tool '{name}' not found")]
    ToolNotFound { name: String },

    #[error("Invalid parameters for tool '{tool}': {message}")]
    InvalidParameters { tool: String, message: String },

    #[error("Tool '{tool}' execution failed: {message}")]
    ExecutionFailed { tool: String, message: String },

    #[error("Schema validation failed for tool '{tool}': {message}")]
    SchemaValidation { tool: String, message: String },

    #[error("Tool registration failed: {message}")]
    RegistrationError { message: String },

    #[error("Serialization error for tool '{tool}': {source}")]
    SerializationError {
        tool: String,
        #[source]
        source: serde_json::Error,
    },

    #[error("Timeout error for tool '{tool}': execution exceeded {timeout:?}")]
    TimeoutError {
        tool: String,
        timeout: std::time::Duration,
    },

    #[error("Retry limit exceeded for tool '{tool}': failed after {attempts} attempts")]
    RetryLimitExceeded { tool: String, attempts: u32 },

    #[error("Validation error for field '{field}': {message}")]
    ValidationError { field: String, message: String },

    #[error("Concurrent access error: {message}")]
    ConcurrentAccessError { message: String },

    #[error("Internal error: {0}")]
    Internal(#[from] anyhow::Error),
}

/// Error context builder for better error reporting
pub struct ErrorContext {
    tool_name: Option<String>,
    operation: Option<String>,
}

impl ErrorContext {
    pub fn new() -> Self {
        Self {
            tool_name: None,
            operation: None,
        }
    }

    pub fn with_tool(mut self, tool_name: impl Into<String>) -> Self {
        self.tool_name = Some(tool_name.into());
        self
    }

    pub fn with_operation(mut self, operation: impl Into<String>) -> Self {
        self.operation = Some(operation.into());
        self
    }

    fn get_tool_name(&self) -> String {
        self.tool_name
            .clone()
            .unwrap_or_else(|| "unknown".to_string())
    }

    pub fn tool_not_found(self) -> ToolError {
        ToolError::ToolNotFound {
            name: self.get_tool_name(),
        }
    }

    pub fn invalid_parameters(self, message: impl Into<String>) -> ToolError {
        let mut msg = message.into();
        if let Some(ref op) = self.operation {
            msg = format!("[{}] {}", op, msg);
        }
        ToolError::InvalidParameters {
            tool: self.get_tool_name(),
            message: msg,
        }
    }

    pub fn execution_failed(self, message: impl Into<String>) -> ToolError {
        let mut msg = message.into();
        if let Some(ref op) = self.operation {
            msg = format!("[{}] {}", op, msg);
        }
        ToolError::ExecutionFailed {
            tool: self.get_tool_name(),
            message: msg,
        }
    }

    pub fn schema_validation(self, message: impl Into<String>) -> ToolError {
        let mut msg = message.into();
        if let Some(ref op) = self.operation {
            msg = format!("[{}] {}", op, msg);
        }
        ToolError::SchemaValidation {
            tool: self.get_tool_name(),
            message: msg,
        }
    }

    pub fn serialization_error(self, source: serde_json::Error) -> ToolError {
        let mut tool_name = self.get_tool_name();
        if let Some(ref op) = self.operation {
            tool_name = format!("{} [{}]", tool_name, op);
        }
        ToolError::SerializationError {
            tool: tool_name,
            source,
        }
    }

    pub fn timeout_error(self, timeout: std::time::Duration) -> ToolError {
        ToolError::TimeoutError {
            tool: self.get_tool_name(),
            timeout,
        }
    }

    pub fn retry_limit_exceeded(self, attempts: u32) -> ToolError {
        ToolError::RetryLimitExceeded {
            tool: self.get_tool_name(),
            attempts,
        }
    }

    pub fn validation_error(
        self,
        field: impl Into<String>,
        message: impl Into<String>,
    ) -> ToolError {
        ToolError::ValidationError {
            field: field.into(),
            message: message.into(),
        }
    }

    pub fn concurrent_access_error(self, message: impl Into<String>) -> ToolError {
        ToolError::ConcurrentAccessError {
            message: message.into(),
        }
    }
}

impl Default for ErrorContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience function to create error context
pub fn error_context() -> ErrorContext {
    ErrorContext::new()
}

