//! Tools module merged from the former `zai-tools` crate.

pub mod core;
pub mod executor;
pub mod error;
pub mod macros;
pub mod llm;

// Built-in tools (optional)
#[cfg(feature = "builtin-tools")]
pub mod builtin;

// Enterprise features (optional)
#[cfg(feature = "config-management")]
pub mod config;

#[cfg(feature = "monitoring")]
pub mod monitoring;

/// Prelude module for convenient imports
pub mod prelude {
    // Core traits and types
    pub use crate::tools::core::{
        Tool, ToolInput, ToolOutput, DynTool, ToolMetadata,
        conversions, FunctionTool,
    };

    // Execution (executor now owns registration APIs)
    pub use crate::tools::executor::{ToolExecutor, ExecutorBuilder, ExecutionResult, ExecutionConfig};

    // Error handling
    pub use crate::tools::error::{ToolError, ToolResult, error_context};

    // Built-in tools
    #[cfg(feature = "builtin-tools")]
    pub use crate::tools::builtin::*;

    // External re-exports for convenience
    pub use async_trait::async_trait;
    pub use serde::{Deserialize, Serialize};

    // Macros (exported at crate root by #[macro_export])
    pub use crate::{simple_tool, async_tool, validated_tool};

    // LLM parsing helpers
    pub use crate::tools::llm::{LlmToolCall, parse_tool_calls, parse_tool_calls_from_message, parse_first_tool_call};
}

// Re-export commonly used types at crate root for convenience via tools::
pub use crate::tools::core::{Tool, ToolInput, ToolOutput, ToolMetadata, FunctionTool};
pub use crate::tools::executor::ToolExecutor;
pub use crate::tools::error::{ToolError, ToolResult};

