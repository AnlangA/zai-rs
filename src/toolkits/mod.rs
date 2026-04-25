//! # Toolkits Module
//!
//! A comprehensive tool-calling and execution framework for AI applications.
//! Supports both static tool definitions and dynamic registration at runtime.
//!
//! # Core Components
//!
//! - [`core`] — Core traits ([`DynTool`], [`FunctionTool`]) and type
//!   conversions
//! - [`error`] — Error types with context information
//! - [`executor`] — Execution engine with registration, caching, and retry
//!   logic
//! - [`llm`] — LLM-specific parsing utilities (tool-call extraction)
//! - [`cache`] — In-memory tool-call cache with statistics
//!
//! # Feature-gated
//!
//! - `rmcp-kits` — RMCP protocol bridge for MCP tool calling
//!
//! # Quick Start
//!
//! ```rust,ignore
//! use zai_rs::toolkits::prelude::*;
//!
//! let tool = FunctionTool::builder("get_weather", "Get current weather")
//!     .property("location", json!({"type": "string"}))
//!     .required("location")
//!     .handler(|input| async move {
//!         Ok(json!({"temperature": 22.5}))
//!     })
//!     .build()?;
//!
//! let mut executor = ToolExecutor::new();
//! executor.register_tool(Box::new(tool))?;
//! ```

pub mod cache;
pub mod core;
pub mod error;
pub mod executor;
pub mod llm;

// RMCP bridge (feature-gated)
#[cfg(feature = "rmcp-kits")]
pub mod rmcp_kits;

/// Prelude module for convenient imports
///
/// This module re-exports commonly used types and traits from the toolkits
/// module, making it easier to import everything needed for tool development
/// with a single `use` statement.
///
/// ## Usage
///
/// ```rust,ignore
/// use zai_rs::toolkits::prelude::*;
/// ```
pub mod prelude {
    // Core traits and types
    // External re-exports for convenience
    pub use async_trait::async_trait;
    pub use serde::{Deserialize, Serialize};

    // Caching
    pub use crate::toolkits::cache::{CacheEntry, CacheKey, CacheStats, ToolCallCache};
    pub use crate::toolkits::core::{DynTool, FunctionTool, ToolMetadata, conversions};
    // Error handling
    pub use crate::toolkits::error::{ToolError, ToolResult, error_context};
    // Execution (executor now owns registration APIs)
    pub use crate::toolkits::executor::{
        ExecutionConfig, ExecutionResult, ExecutorBuilder, ToolExecutor,
    };
    // LLM parsing helpers
    pub use crate::toolkits::llm::{
        LlmToolCall, parse_first_tool_call, parse_tool_calls, parse_tool_calls_from_message,
    };
    // RMCP bridge exports when enabled
    #[cfg(feature = "rmcp-kits")]
    pub use crate::toolkits::rmcp_kits::{
        McpToolCaller, call_mcp_tool, call_mcp_tools_collect, call_tool_result_to_json,
        mcp_tool_to_function, mcp_tools_to_functions,
    };
}

// Re-export commonly used types at crate root for convenience via toolkits::
pub use crate::toolkits::{
    core::{FunctionTool, ToolMetadata},
    error::{ToolError, ToolResult},
    executor::ToolExecutor,
};
