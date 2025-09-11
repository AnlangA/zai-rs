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
//! - [`macros`] - Procedural macros for tool creation
//!
//! ## Key Features
//!
//! ### Type Safety
//! - Compile-time validation of tool inputs and outputs
//! - Strong typing for tool parameters and return values
//! - Generic tool definitions with trait bounds
//!
//! ### Flexibility
//! - Support for both sync and async tool execution
//! - Dynamic tool registration and discovery
//! - JSON schema generation for tool descriptions
//!
//! ### Error Handling
//! - Comprehensive error types with context information
//! - Graceful error recovery and reporting
//! - Validation errors with detailed messages
//!
//! ## Usage Examples
//!
//! ### Basic Tool Definition
//! ```rust,ignore
//! use zai_rs::toolkits::prelude::*;
//!
//! #[derive(ToolInput, Deserialize)]
//! struct WeatherInput {
//!     location: String,
//! }
//!
//! #[derive(ToolOutput, Serialize)]
//! struct WeatherOutput {
//!     temperature: f32,
//!     condition: String,
//! }
//!
//! struct WeatherTool;
//!
//! #[async_trait]
//! impl Tool<WeatherInput, WeatherOutput> for WeatherTool {
//!     async fn execute(&self, input: WeatherInput) -> ToolResult<WeatherOutput> {
//!         // Tool implementation
//!         Ok(WeatherOutput {
//!             temperature: 22.5,
//!             condition: "Sunny".to_string(),
//!         })
//!     }
//! }
//! ```
//!
//! ### Function Tool Creation
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
pub mod macros;

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
    pub use crate::toolkits::core::{
        conversions, DynTool, FunctionTool, Tool, ToolInput, ToolMetadata, ToolOutput,
    };

    // Execution (executor now owns registration APIs)
    pub use crate::toolkits::executor::{
        ExecutionConfig, ExecutionResult, ExecutorBuilder, ToolExecutor,
    };

    // Error handling
    pub use crate::toolkits::error::{error_context, ToolError, ToolResult};

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
