//! Error types and context builders for tool operations.

use std::borrow::Cow;

use thiserror::Error;

/// Result type for tool operations
pub type ToolResult<T> = Result<T, ToolError>;

/// Error returned while registering, validating, or executing a tool.
#[derive(Error, Debug)]
#[non_exhaustive]
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
    ///
    /// Note: the timeout cancels only the *local* execution future. For a tool
    /// backed by a remote service (e.g. an MCP tool, or any tool that performs
    /// I/O), the request may already be on the wire and the remote side effect
    /// may still occur. A timed-out call is not cached, and because
    /// `TimeoutError` is retryable the same call may be issued more than once —
    /// so keep per-call timeouts conservative for non-idempotent tools.
    #[error("Timeout error for tool '{tool}': execution exceeded {timeout:?}")]
    TimeoutError {
        /// Name of the tool.
        tool: Cow<'static, str>,
        /// Configured timeout that was exceeded.
        timeout: std::time::Duration,
    },

    /// A tool handler panicked while it was being polled.
    ///
    /// Panic payloads are deliberately omitted because they may contain
    /// sensitive data and are not a stable error interface.
    #[error("Tool '{tool}' execution panicked")]
    ExecutionPanicked {
        /// Name of the tool whose handler panicked.
        tool: Cow<'static, str>,
    },
}

impl ToolError {
    /// Determine if the error is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            ToolError::TimeoutError { .. } | ToolError::ExecutionFailed { .. }
        )
    }
}

/// Builder that attaches tool and operation context to a [`ToolError`].
pub struct ErrorContext {
    tool_name: Option<String>,
}

impl ErrorContext {
    /// Create a new empty error context.
    pub fn new() -> Self {
        Self { tool_name: None }
    }

    /// Attach a tool name to this context.
    pub fn with_tool(mut self, tool_name: impl Into<String>) -> Self {
        self.tool_name = Some(tool_name.into());
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
        ToolError::InvalidParameters {
            tool: Cow::Owned(self.get_tool_name()),
            message: Cow::Owned(message.into()),
        }
    }

    /// Build a [`ToolError::ExecutionFailed`] from this context.
    pub fn execution_failed(self, message: impl Into<String>) -> ToolError {
        ToolError::ExecutionFailed {
            tool: Cow::Owned(self.get_tool_name()),
            message: Cow::Owned(message.into()),
        }
    }

    /// Build a [`ToolError::SchemaValidation`] from this context.
    pub fn schema_validation(self, message: impl Into<String>) -> ToolError {
        ToolError::SchemaValidation {
            tool: Cow::Owned(self.get_tool_name()),
            message: Cow::Owned(message.into()),
        }
    }

    /// Build a [`ToolError::SerializationError`] from this context.
    pub fn serialization_error(self, source: serde_json::Error) -> ToolError {
        ToolError::SerializationError {
            tool: Cow::Owned(self.get_tool_name()),
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
