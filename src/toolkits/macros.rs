//! Macro support and helper functions
//! 
//! This module provides declarative macros for simplified tool creation
//! and helper functions for tool registration and management.
//! 
//! Note: Procedural macros have been removed to avoid the complexity of
//! mixed crate types. Use the declarative macros for tool creation.

/// Create a simple tool from a closure
#[macro_export]
macro_rules! simple_tool {
    (
        name: $name:expr,
        description: $desc:expr,
        input: $input_type:ty,
        output: $output_type:ty,
        execute: $execute_fn:expr
    ) => {
        {
            use $crate::toolkits::core::*;
            use $crate::toolkits::error::*;
            
            #[derive(Clone)]
            struct SimpleTool {
                metadata: ToolMetadata,
            }
            
            impl SimpleTool {
                fn new() -> Self {
                    Self {
                        metadata: ToolMetadata::new::<$input_type, $output_type>($name, $desc),
                    }
                }
            }
            
            #[$crate::toolkits::prelude::async_trait]
            impl Tool<$input_type, $output_type> for SimpleTool {
                fn metadata(&self) -> &ToolMetadata {
                    &self.metadata
                }

                async fn execute(&self, input: $input_type) -> ToolResult<$output_type> {
                    let execute_fn: fn($input_type) -> ToolResult<$output_type> = $execute_fn;
                    execute_fn(input)
                }
            }
            
            SimpleTool::new()
        }
    };
}

/// Helper macro for creating async tools
#[macro_export]
macro_rules! async_tool {
    (
        name: $name:expr,
        description: $desc:expr,
        input: $input_type:ty,
        output: $output_type:ty,
        execute: $execute_fn:expr
    ) => {
        {
            use $crate::toolkits::core::*;
            use $crate::toolkits::error::*;
            
            #[derive(Clone)]
            struct AsyncTool {
                metadata: ToolMetadata,
            }
            
            impl AsyncTool {
                fn new() -> Self {
                    Self {
                        metadata: ToolMetadata::new::<$input_type, $output_type>($name, $desc),
                    }
                }
            }
            
            #[$crate::toolkits::prelude::async_trait]
            impl Tool<$input_type, $output_type> for AsyncTool {
                fn metadata(&self) -> &ToolMetadata {
                    &self.metadata
                }

                async fn execute(&self, input: $input_type) -> ToolResult<$output_type> {
                    let execute_fn: fn($input_type) -> std::pin::Pin<Box<dyn std::future::Future<Output = ToolResult<$output_type>> + Send>> = $execute_fn;
                    execute_fn(input).await
                }
            }
            
            AsyncTool::new()
        }
    };
}

/// Helper macro for creating tools with validation
#[macro_export]
macro_rules! validated_tool {
    (
        name: $name:expr,
        description: $desc:expr,
        input: $input_type:ty,
        output: $output_type:ty,
        validate: $validate_fn:expr,
        execute: $execute_fn:expr
    ) => {
        {
            use $crate::toolkits::core::*;
            use $crate::toolkits::error::*;
            
            #[derive(Clone)]
            struct ValidatedTool {
                metadata: ToolMetadata,
            }
            
            impl ValidatedTool {
                fn new() -> Self {
                    Self {
                        metadata: ToolMetadata::new::<$input_type, $output_type>($name, $desc),
                    }
                }
            }
            
            #[$crate::toolkits::prelude::async_trait]
            impl Tool<$input_type, $output_type> for ValidatedTool {
                fn metadata(&self) -> &ToolMetadata {
                    &self.metadata
                }

                async fn execute(&self, input: $input_type) -> ToolResult<$output_type> {
                    // Validate input first
                    let validate_fn: fn(&$input_type) -> ToolResult<()> = $validate_fn;
                    validate_fn(&input)?;

                    // Execute the tool
                    let execute_fn: fn($input_type) -> ToolResult<$output_type> = $execute_fn;
                    execute_fn(input)
                }
            }
            
            ValidatedTool::new()
        }
    };
}

#[cfg(test)]
mod tests {

    use crate::toolkits::prelude::*;
    use serde::{Deserialize, Serialize};
    
    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct TestInput {
        value: i32,
    }
    
    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct TestOutput {
        result: i32,
    }
    
    impl ToolInput for TestInput {}
    impl ToolOutput for TestOutput {}
    
    #[tokio::test]
    async fn test_simple_tool_macro() {
        let tool = simple_tool! {
            name: "test_tool",
            description: "A test tool",
            input: TestInput,
            output: TestOutput,
            execute: |input: TestInput| -> ToolResult<TestOutput> {
                Ok(TestOutput { result: input.value * 2 })
            }
        };
        
        let input = TestInput { value: 5 };
        let output = tool.execute(input).await.unwrap();
        assert_eq!(output.result, 10);
    }
}

