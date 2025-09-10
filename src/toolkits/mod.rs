//! Toolkits module merged from the former `zai-tools` crate.

pub mod core;
pub mod error;
pub mod executor;
pub mod llm;
pub mod macros;

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
    pub use crate::toolkits::core::{
        conversions, DynTool, FunctionTool, Tool, ToolInput, ToolMetadata, ToolOutput,
    };

    // Execution (executor now owns registration APIs)
    pub use crate::toolkits::executor::{
        ExecutionConfig, ExecutionResult, ExecutorBuilder, ToolExecutor,
    };

    // Error handling
    pub use crate::toolkits::error::{error_context, ToolError, ToolResult};

    // Built-in tools
    #[cfg(feature = "builtin-tools")]
    pub use crate::toolkits::builtin::*;

    // External re-exports for convenience
    pub use async_trait::async_trait;
    pub use serde::{Deserialize, Serialize};

    // Macros (exported at crate root by #[macro_export])
    pub use crate::{async_tool, simple_tool, validated_tool};

    // LLM parsing helpers
    pub use crate::toolkits::llm::{
        parse_first_tool_call, parse_tool_calls, parse_tool_calls_from_message, LlmToolCall,
    };
}

// Re-export commonly used types at crate root for convenience via toolkits::
pub use crate::toolkits::core::{FunctionTool, Tool, ToolInput, ToolMetadata, ToolOutput};
pub use crate::toolkits::error::{ToolError, ToolResult};
pub use crate::toolkits::executor::ToolExecutor;
