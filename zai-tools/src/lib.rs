//! # ZAI Tools
//!
//! A powerful and flexible tool system for AI function calling with enhanced type safety.
//!
//! This crate provides a comprehensive framework for defining, registering, and executing
//! tools that can be called by AI models. It features:
//!
//! - **Type-safe tool definitions** with automatic schema generation
//! - **Async tool execution** with proper error handling and timeouts
//! - **Plugin-style tool registration** system with dependency injection
//! - **Built-in tools** for common use cases (HTTP, file operations, etc.)
//! - **Macro support** for simplified tool creation
//! - **Performance optimized** with parallel execution support
//! - **Enterprise-ready** with logging, monitoring, and configuration management
//!
//! ## Quick Start
//!
//! ```rust
//! use zai_tools::prelude::*;
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Debug, Clone, Serialize, Deserialize)]
//! struct CalculatorInput {
//!     operation: String,
//!     a: f64,
//!     b: f64,
//! }
//!
//! #[derive(Debug, Clone, Serialize, Deserialize)]
//! struct CalculatorOutput {
//!     result: f64,
//! }
//!
//! impl ToolInput for CalculatorInput {}
//! impl ToolOutput for CalculatorOutput {}
//!
//! #[derive(Clone)]
//! struct Calculator {
//!     metadata: ToolMetadata,
//! }
//!
//! impl Calculator {
//!     fn new() -> Self {
//!         Self {
//!             metadata: ToolMetadata::new::<CalculatorInput, CalculatorOutput>(
//!                 "calculator",
//!                 "A simple calculator tool"
//!             ),
//!         }
//!     }
//! }
//!
//! #[async_trait]
//! impl Tool<CalculatorInput, CalculatorOutput> for Calculator {
//!     fn metadata(&self) -> &ToolMetadata {
//!         &self.metadata
//!     }
//!
//!     async fn execute(&self, input: CalculatorInput) -> ToolResult<CalculatorOutput> {
//!         let result = match input.operation.as_str() {
//!             "add" => input.a + input.b,
//!             "subtract" => input.a - input.b,
//!             "multiply" => input.a * input.b,
//!             "divide" => input.a / input.b,
//!             _ => return Err(error_context().invalid_parameters("Invalid operation")),
//!         };
//!         Ok(CalculatorOutput { result })
//!     }
//! }
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a tool registry and register tools
//! let registry = ToolRegistry::builder()
//!     .with_tool(Calculator::new())?
//!     .build();
//!
//! // Create an executor
//! let executor = ToolExecutor::builder(registry)
//!     .timeout(std::time::Duration::from_secs(30))
//!     .build();
//!
//! // Execute a tool
//! let input = serde_json::json!({
//!     "operation": "add",
//!     "a": 10.0,
//!     "b": 20.0
//! });
//! let result = executor.execute("calculator", input).await?;
//! println!("Result: {}", serde_json::to_string_pretty(&result.result)?);
//! # Ok(())
//! # }
//! ```

// Core modules
pub mod core;
pub mod registry;
pub mod executor;
pub mod error;
pub mod macros;

// Built-in tools
#[cfg(feature = "builtin-tools")]
pub mod builtin;

// Enterprise features
#[cfg(feature = "config-management")]
pub mod config;

#[cfg(feature = "monitoring")]
pub mod monitoring;

/// Prelude module for convenient imports
pub mod prelude {
    //! Common imports for using zai-tools
    //!
    //! This module re-exports the most commonly used types and traits,
    //! making it easy to get started with zai-tools.

    // Core traits and types
    pub use crate::core::{
        Tool, ToolInput, ToolOutput, DynTool, ToolWrapper, ToolMetadata,
        IntoDynTool, conversions,
    };

    // Registry and execution
    pub use crate::registry::{ToolRegistry, RegistryBuilder};
    pub use crate::executor::{ToolExecutor, ExecutorBuilder, ExecutionResult, ExecutionConfig};

    // Error handling
    pub use crate::error::{ToolError, ToolResult, error_context};

    // Built-in tools
    #[cfg(feature = "builtin-tools")]
    pub use crate::builtin::*;

    // External re-exports for convenience
    pub use async_trait::async_trait;
    pub use serde::{Deserialize, Serialize};

    // Macros
    pub use crate::{simple_tool, async_tool, validated_tool};
}

// Re-export commonly used types at crate root for convenience
pub use crate::core::{Tool, ToolInput, ToolOutput, ToolMetadata};
pub use crate::registry::ToolRegistry;
pub use crate::executor::ToolExecutor;
pub use crate::error::{ToolError, ToolResult};
