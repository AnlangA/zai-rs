//! Enhanced error handling with better Rust idioms

use thiserror::Error;
use std::borrow::Cow;

/// Result type for tool operations
pub type ToolResult<T> = Result<T, ToolError>;

/// Error severity levels for better error handling strategies
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorSeverity {
    User,      // User error, no retry needed
    Normal,    // Normal error, may retry
    Transient, // Transient error, should retry
    Critical,  // Critical error, log and alert
}

/// Enhanced error type with better context and error chaining
#[derive(Error, Debug)]
pub enum ToolError {
    #[error("Tool '{name}' not found")]
    ToolNotFound { name: Cow<'static, str> },

    #[error("Invalid parameters for tool '{tool}': {message}")]
    InvalidParameters { tool: Cow<'static, str>, message: Cow<'static, str> },

    #[error("Tool '{tool}' execution failed: {message}")]
    ExecutionFailed { tool: Cow<'static, str>, message: Cow<'static, str> },

    #[error("Schema validation failed for tool '{tool}': {message}")]
    SchemaValidation { tool: Cow<'static, str>, message: Cow<'static, str> },

    #[error("Tool registration failed: {message}")]
    RegistrationError { message: Cow<'static, str> },

    #[error("Serialization error for tool '{tool}': {source}")]
    SerializationError {
        tool: Cow<'static, str>,
        #[source]
        source: serde_json::Error,
    },

    #[error("Timeout error for tool '{tool}': execution exceeded {timeout:?}")]
    TimeoutError {
        tool: Cow<'static, str>,
        timeout: std::time::Duration,
    },

    #[error("Retry limit exceeded for tool '{tool}': failed after {attempts} attempts")]
    RetryLimitExceeded { tool: Cow<'static, str>, attempts: u32 },

    #[error("Validation error for field '{field}': {message}")]
    ValidationError { field: Cow<'static, str>, message: Cow<'static, str> },

    #[error("Concurrent access error: {message}")]
    ConcurrentAccessError { message: Cow<'static, str> },

    #[error("Internal error: {0}")]
    Internal(#[from] anyhow::Error),
}

impl ToolError {
    /// Determine if the error is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(self, 
            ToolError::TimeoutError { .. } | 
            ToolError::ConcurrentAccessError { .. } |
            ToolError::ExecutionFailed { .. }
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
            name: Cow::Owned(self.get_tool_name()),
        }
    }

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

    pub fn timeout_error(self, timeout: std::time::Duration) -> ToolError {
        ToolError::TimeoutError {
            tool: Cow::Owned(self.get_tool_name()),
            timeout,
        }
    }

    pub fn retry_limit_exceeded(self, attempts: u32) -> ToolError {
        ToolError::RetryLimitExceeded {
            tool: Cow::Owned(self.get_tool_name()),
            attempts,
        }
    }

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

