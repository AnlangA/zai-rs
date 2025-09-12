//! # Toolkits Module
//!
//! A comprehensive tool calling and execution framework for AI applications.
//! This module was merged from the former `zai-tools` crate and provides
//! a robust foundation for integrating external tools and functions with AI models.
//!
//! ## Overview
//!
//! The toolkits module enables AI models to invoke external functions, APIs, and tools
//! in a type-safe and extensible manner. It supports both static tool definitions
//! and dynamic tool registration at runtime.
//!
//! ## Core Components
//!
//! - [`core`] - Core traits and types for tool definitions
//! - [`error`] - Comprehensive error handling and reporting
//! - [`executor`] - Tool execution engine with registration and management
//! - [`llm`] - LLM-specific parsing and interaction utilities
//!
//! ## Key Features
//!
//! ### Flexibility
//! - Dynamic tool registration and discovery
//! - JSON schema generation for tool descriptions
//!
//! ### Error Handling
//! - Comprehensive error types with context information
//! - Graceful error recovery and reporting
//! - Validation errors with detailed messages
//!
//! ## Usage Example
//! ```rust,ignore
//! let tool = FunctionTool::builder("get_weather", "Get current weather")
//!     .property("location", json!({"type": "string"}))
//!     .required("location")
//!     .handler(|input| async move {
//!         // Function implementation
//!         Ok(json!({"temperature": 22.5}))
//!     })
//!     .build()?;
//! ```

pub mod core;
pub mod error;
pub mod executor;
pub mod llm;

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
    pub use crate::toolkits::core::{conversions, DynTool, FunctionTool, ToolMetadata};

    // Execution (executor now owns registration APIs)
    pub use crate::toolkits::executor::{
        ExecutionConfig, ExecutionResult, ExecutorBuilder, ToolExecutor,
    };

    // Error handling
    pub use crate::toolkits::error::{error_context, ToolError, ToolResult};

    // External re-exports for convenience
    pub use async_trait::async_trait;
    pub use serde::{Deserialize, Serialize};

    // LLM parsing helpers
    pub use crate::toolkits::llm::{
        parse_first_tool_call, parse_tool_calls, parse_tool_calls_from_message, LlmToolCall,
    };
}

// Re-export commonly used types at crate root for convenience via toolkits::
pub use crate::toolkits::core::{FunctionTool, ToolMetadata};
pub use crate::toolkits::error::{ToolError, ToolResult};
pub use crate::toolkits::executor::ToolExecutor;
