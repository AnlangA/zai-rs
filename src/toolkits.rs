// The `core::DynTool` / `core::FunctionTool` intra-doc links here are necessary
// on stable (a bare `DynTool` does not resolve because `core` is ambiguous with
// the std `core` crate). On nightly, `rustdoc::redundant_explicit_links` flags
// them anyway (a known false-positive under the ambiguity); allow it locally.
#![allow(rustdoc::redundant_explicit_links)]

//! # Toolkits Module
//!
//! Tool definition, execution, caching, and LLM tool-call parsing utilities.
//! Supports both static tool definitions and dynamic registration at runtime.
//!
//! # Core Components
//!
//! - [`core`] — Core traits ([`DynTool`](core::DynTool), [`FunctionTool`]) and
//!   type conversions
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
//! ```no_run
//! use zai_rs::toolkits::prelude::*;
//! use serde_json::json;
//!
//! # fn main() -> ToolResult<()> {
//! let tool = FunctionTool::builder("get_weather", "Get current weather")
//!     .property("location", json!({"type": "string"}))
//!     .required("location")
//!     .handler(|input| async move {
//!         Ok(json!({"temperature": 22.5}))
//!     })
//!     .build()?;
//!
//! let executor = ToolExecutor::new();
//! executor.add_dyn_tool(Box::new(tool))?;
//! # Ok(())
//! # }
//! ```

/// In-memory tool-call cache with per-entry hit statistics.
pub mod cache;
/// Core traits ([`DynTool`](core::DynTool),
/// [`FunctionTool`](core::FunctionTool)) and type conversions.
pub mod core;
/// Error types with context information.
pub mod error;
/// Execution engine with registration, caching, and retry logic.
pub mod executor;
/// LLM-specific parsing utilities (tool-call extraction).
pub mod llm;

// RMCP bridge (feature-gated)
/// RMCP protocol bridge for MCP tool calling (feature `rmcp-kits`).
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
/// ```
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
    #[cfg(feature = "mcp")]
    pub use crate::mcp::*;
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
