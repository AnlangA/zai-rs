//! Built-in tools for common use cases
//!
//! This module provides a comprehensive set of built-in tools that cover
//! common use cases for AI function calling. All tools are implemented
//! with type safety and proper error handling.

// Core tools
pub mod calculator;

// Re-export all tools
pub use calculator::*;

use crate::registry::ToolRegistry;
use crate::error::ToolResult;

/// Register all available built-in tools to a registry
pub fn register_all_builtin_tools(registry: &ToolRegistry) -> ToolResult<()> {
    // Core tools (always available)
    registry.register(CalculatorTool::new())?;
    Ok(())
}

/// Register only core built-in tools (no optional dependencies)
pub fn register_core_builtin_tools(registry: &ToolRegistry) -> ToolResult<()> {
    registry.register(CalculatorTool::new())?;
    Ok(())
}

/// Convenience alias for the main registration function
pub fn register_builtin_tools(registry: &ToolRegistry) -> ToolResult<()> {
    register_all_builtin_tools(registry)
}
