//! Enhanced error handling with better Rust idioms

use std::borrow::Cow;

use thiserror::Error;

/// Result type for tool operations
pub type ToolResult<T> = Result<T, ToolError>;

/// Error severity levels for better error handling strategies
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorSeverity {
    /// User error, no retry needed.
    User, // User error, no retry needed
    /// Normal error, may retry.
    Normal, // Normal error, may retry
    /// Transient error, should retry.
    Transient, // Transient error, should retry
    /// Critical error, log and alert.
    Critical, // Critical error, log and alert
}

/// Enhanced error type with better context and error chaining
#[derive(Error, Debug)]
pub enum ToolError {
    /// The requested tool is not registered.
    #[error("Tool '{name}' not found")]
    ToolNotFound {
        /// Name of the missing tool.
        name: Cow<'static, str>,
    },

    /// The supplied parameters were invalid for the tool.
    #[error("Invalid parameters for tool '{tool}': {message}")]
    InvalidParameters {
        /// Name of the tool.
        tool: Cow<'static, str>,
        /// Why the parameters were invalid.
        message: Cow<'static, str>,
    },

    /// Tool execution failed at runtime.
    #[error("Tool '{tool}' execution failed: {message}")]
    ExecutionFailed {
        /// Name of the tool.
        tool: Cow<'static, str>,
        /// What went wrong during execution.
        message: Cow<'static, str>,
    },

    /// The tool's JSON schema validation failed.
    #[error("Schema validation failed for tool '{tool}': {message}")]
    SchemaValidation {
        /// Name of the tool.
        tool: Cow<'static, str>,
        /// Schema-validation error detail.
        message: Cow<'static, str>,
    },

    /// Tool registration failed (duplicate name, …).
    #[error("Tool registration failed: {message}")]
    RegistrationError {
        /// Why registration failed.
        message: Cow<'static, str>,
    },

    /// (De)serialization for a tool payload failed.
    #[error("Serialization error for tool '{tool}': {source}")]
    SerializationError {
        /// Name of the tool.
        tool: Cow<'static, str>,
        /// Underlying serde error.
        #[source]
        source: serde_json::Error,
    },

    /// Tool execution exceeded its timeout.
    #[error("Timeout error for tool '{tool}': execution exceeded {timeout:?}")]
    TimeoutError {
        /// Name of the tool.
        tool: Cow<'static, str>,
        /// Configured timeout that was exceeded.
        timeout: std::time::Duration,
    },

    /// Retries were exhausted before the tool succeeded.
    #[error("Retry limit exceeded for tool '{tool}': failed after {attempts} attempts")]
    RetryLimitExceeded {
        /// Name of the tool.
        tool: Cow<'static, str>,
        /// Number of attempts made.
        attempts: u32,
    },

    /// A field-level validation error.
    #[error("Validation error for field '{field}': {message}")]
    ValidationError {
        /// Name of the invalid field.
        field: Cow<'static, str>,
        /// Why the field was invalid.
        message: Cow<'static, str>,
    },

    /// A concurrency/locking conflict during execution.
    #[error("Concurrent access error: {message}")]
    ConcurrentAccessError {
        /// Detail of the conflict.
        message: Cow<'static, str>,
    },

    /// Catch-all internal SDK error.
    #[error("Internal error: {0}")]
    Internal(String),
}

impl ToolError {
    /// Determine if the error is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            ToolError::TimeoutError { .. }
                | ToolError::ConcurrentAccessError { .. }
                | ToolError::ExecutionFailed { .. }
        )
    }

    /// Get the severity level of the error
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            ToolError::ToolNotFound { .. } => ErrorSeverity::User,
            ToolError::InvalidParameters { .. } => ErrorSeverity::User,
            ToolError::ValidationError { .. } => ErrorSeverity::User,
            ToolError::TimeoutError { .. } => ErrorSeverity::Transient,
            ToolError::ConcurrentAccessError { .. } => ErrorSeverity::Transient,
            ToolError::Internal(_) => ErrorSeverity::Critical,
            _ => ErrorSeverity::Normal,
        }
    }
}

/// Error context builder for better error reporting
pub struct ErrorContext {
    tool_name: Option<String>,
    operation: Option<String>,
}

impl ErrorContext {
    /// Create a new empty error context.
    pub fn new() -> Self {
        Self {
            tool_name: None,
            operation: None,
        }
    }

    /// Attach a tool name to this context.
    pub fn with_tool(mut self, tool_name: impl Into<String>) -> Self {
        self.tool_name = Some(tool_name.into());
        self
    }

    /// Attach an operation name (e.g. `"execute"`, `"validate"`) to this
    /// context.
    pub fn with_operation(mut self, operation: impl Into<String>) -> Self {
        self.operation = Some(operation.into());
        self
    }

    fn get_tool_name(&self) -> String {
        self.tool_name
            .clone()
            .unwrap_or_else(|| "unknown".to_string())
    }

    /// Build a [`ToolError::ToolNotFound`] from this context.
    pub fn tool_not_found(self) -> ToolError {
        ToolError::ToolNotFound {
            name: Cow::Owned(self.get_tool_name()),
        }
    }

    /// Build a [`ToolError::InvalidParameters`] from this context.
    pub fn invalid_parameters(self, message: impl Into<String>) -> ToolError {
        let mut msg = message.into();
        if let Some(ref op) = self.operation {
            msg = format!("[{}] {}", op, msg);
        }
        ToolError::InvalidParameters {
            tool: Cow::Owned(self.get_tool_name()),
            message: Cow::Owned(msg),
        }
    }

    /// Build a [`ToolError::ExecutionFailed`] from this context.
    pub fn execution_failed(self, message: impl Into<String>) -> ToolError {
        let mut msg = message.into();
        if let Some(ref op) = self.operation {
            msg = format!("[{}] {}", op, msg);
        }
        ToolError::ExecutionFailed {
            tool: Cow::Owned(self.get_tool_name()),
            message: Cow::Owned(msg),
        }
    }

    /// Build a [`ToolError::SchemaValidation`] from this context.
    pub fn schema_validation(self, message: impl Into<String>) -> ToolError {
        let mut msg = message.into();
        if let Some(ref op) = self.operation {
            msg = format!("[{}] {}", op, msg);
        }
        ToolError::SchemaValidation {
            tool: Cow::Owned(self.get_tool_name()),
            message: Cow::Owned(msg),
        }
    }

    /// Build a [`ToolError::SerializationError`] from this context.
    pub fn serialization_error(self, source: serde_json::Error) -> ToolError {
        let mut tool_name = self.get_tool_name();
        if let Some(ref op) = self.operation {
            tool_name = format!("{} [{}]", tool_name, op);
        }
        ToolError::SerializationError {
            tool: Cow::Owned(tool_name),
            source,
        }
    }

    /// Build a [`ToolError::TimeoutError`] from this context.
    pub fn timeout_error(self, timeout: std::time::Duration) -> ToolError {
        ToolError::TimeoutError {
            tool: Cow::Owned(self.get_tool_name()),
            timeout,
        }
    }

    /// Build a [`ToolError::RetryLimitExceeded`] from this context.
    pub fn retry_limit_exceeded(self, attempts: u32) -> ToolError {
        ToolError::RetryLimitExceeded {
            tool: Cow::Owned(self.get_tool_name()),
            attempts,
        }
    }

    /// Build a [`ToolError::ValidationError`] from this context.
    pub fn validation_error(
        self,
        field: impl Into<String>,
        message: impl Into<String>,
    ) -> ToolError {
        ToolError::ValidationError {
            field: Cow::Owned(field.into()),
            message: Cow::Owned(message.into()),
        }
    }

    /// Build a [`ToolError::ConcurrentAccessError`] from this context.
    pub fn concurrent_access_error(self, message: impl Into<String>) -> ToolError {
        ToolError::ConcurrentAccessError {
            message: Cow::Owned(message.into()),
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
