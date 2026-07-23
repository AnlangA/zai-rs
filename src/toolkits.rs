//! Tool definition, execution, caching, and LLM tool-call parsing utilities.
//! Supports both static tool definitions and dynamic registration at runtime.
//!
//! # Core Components
//!
//! - [`core`] — Core traits such as the [dynamic tool trait][self::core::DynTool]
//!   and [`FunctionTool`], plus type conversions
//! - [`error`] — Error types with context information
//! - [`executor`] — Execution engine with registration, caching, and retry
//!   logic
//! - [`llm`] — LLM-specific parsing utilities (tool-call extraction)
//! - [`CacheStats`] — Executor cache statistics
//!
//! # Feature-gated
//!
//! - `rmcp-kits` — RMCP protocol bridge for MCP tool calling
//!
//! # Quick Start
//!
//! ```rust,no_run
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

mod cache;
/// Core traits such as the [dynamic tool trait][core::DynTool] and
/// [`FunctionTool`], plus type conversions.
pub mod core;
/// Error types with context information.
pub mod error;
/// Execution engine with registration, caching, and retry logic.
pub mod executor;
/// LLM-specific parsing utilities (tool-call extraction).
pub mod llm;

pub use cache::CacheStats;

/// RMCP protocol bridge for MCP tool calling (feature `rmcp-kits`).
#[cfg(feature = "rmcp-kits")]
#[cfg_attr(docsrs, doc(cfg(feature = "rmcp-kits")))]
pub mod rmcp_kits;

/// Common types and traits used when defining or executing tools.
///
/// # Examples
///
/// ```
/// use zai_rs::toolkits::prelude::*;
/// ```
pub mod prelude {
    pub use async_trait::async_trait;
    pub use serde::{Deserialize, Serialize};

    #[cfg(feature = "mcp")]
    pub use crate::mcp::*;
    pub use crate::toolkits::CacheStats;
    pub use crate::toolkits::core::{
        DynTool, FunctionTool, ToolHandler, ToolMetadata, conversions,
    };
    pub use crate::toolkits::error::{ToolError, ToolResult, error_context};
    pub use crate::toolkits::executor::{
        ExecutionConfig, ExecutionResult, ExecutorBuilder, ToolExecutor,
    };
    pub use crate::toolkits::llm::{
        LlmToolCall, parse_first_tool_call, parse_tool_calls, parse_tool_calls_from_message,
    };
    #[cfg(feature = "rmcp-kits")]
    pub use crate::toolkits::rmcp_kits::{
        McpToolCaller, call_mcp_tool, call_mcp_tools_collect, call_tool_result_to_json,
        mcp_tool_to_function, mcp_tools_to_functions,
    };
}

pub use crate::toolkits::{
    core::{FunctionTool, ToolMetadata},
    error::{ToolError, ToolResult},
    executor::ToolExecutor,
};
