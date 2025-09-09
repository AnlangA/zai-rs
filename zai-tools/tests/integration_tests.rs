//! Integration tests for zai-tools
//! 
//! These tests verify that the entire system works together correctly,
//! including tool registration, execution, error handling, and type safety.

use serde::{Deserialize, Serialize};
use zai_tools::prelude::*;

// Test input/output types
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TestInput {
    value: i32,
    operation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TestOutput {
    result: i32,
    message: String,
}

impl ToolInput for TestInput {
    fn validate(&self) -> ToolResult<()> {
        if self.value < 0 {
            return Err(error_context().invalid_parameters("Value must be non-negative"));
        }
        match self.operation.as_str() {
            "double" | "square" | "increment" => Ok(()),
            _ => Err(error_context().invalid_parameters("Invalid operation")),
        }
    }
    
    fn schema() -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "value": {
                    "type": "integer",
                    "minimum": 0,
                    "description": "Input value"
                },
                "operation": {
                    "type": "string",
                    "enum": ["double", "square", "increment"],
                    "description": "Operation to perform"
                }
            },
            "required": ["value", "operation"]
        })
    }
}

impl ToolOutput for TestOutput {}

// Test tool implementation
#[derive(Clone)]
struct TestTool {
    metadata: ToolMetadata,
}

impl TestTool {
    fn new() -> Self {
        Self {
            metadata: ToolMetadata::new::<TestInput, TestOutput>(
                "test_tool",
                "A test tool for integration testing"
            )
            .version("1.0.0")
            .author("Test Suite")
            .tags(["test", "integration"]),
        }
    }
}

#[async_trait]
impl Tool<TestInput, TestOutput> for TestTool {
    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }
    
    async fn execute(&self, input: TestInput) -> ToolResult<TestOutput> {
        let result = match input.operation.as_str() {
            "double" => input.value * 2,
            "square" => input.value * input.value,
            "increment" => input.value + 1,
            _ => unreachable!(), // Validation should catch this
        };
        
        Ok(TestOutput {
            result,
            message: format!("Applied {} to {}", input.operation, input.value),
        })
    }
}

#[tokio::test]
async fn test_basic_tool_registration_and_execution() {
    let registry = ToolRegistry::builder()
        .with_tool(TestTool::new())
        .unwrap()
        .build();
    
    let executor = ToolExecutor::builder(registry)
        .timeout(std::time::Duration::from_secs(5))
        .build();
    
    let input = serde_json::json!({
        "value": 5,
        "operation": "double"
    });
    
    let result = executor.execute("test_tool", input).await.unwrap();
    assert!(result.success);
    
    let output: TestOutput = serde_json::from_value(result.result).unwrap();
    assert_eq!(output.result, 10);
    assert_eq!(output.message, "Applied double to 5");
}

#[tokio::test]
async fn test_validation_error() {
    let registry = ToolRegistry::builder()
        .with_tool(TestTool::new())
        .unwrap()
        .build();
    
    let executor = ToolExecutor::builder(registry).build();
    
    let input = serde_json::json!({
        "value": -1,
        "operation": "double"
    });
    
    let result = executor.execute("test_tool", input).await.unwrap();
    assert!(!result.success);
    assert!(result.error.is_some());
}

#[tokio::test]
async fn test_tool_not_found() {
    let registry = ToolRegistry::new();
    let executor = ToolExecutor::builder(registry).build();
    
    let input = serde_json::json!({
        "value": 5,
        "operation": "double"
    });
    
    let result = executor.execute("nonexistent_tool", input).await.unwrap();
    assert!(!result.success);
    assert!(result.error.unwrap().contains("not found"));
}

#[tokio::test]
async fn test_parallel_execution() {
    let registry = ToolRegistry::builder()
        .with_tool(TestTool::new())
        .unwrap()
        .build();
    
    let executor = ToolExecutor::builder(registry).build();
    
    let requests = vec![
        ("test_tool".to_string(), serde_json::json!({"value": 1, "operation": "double"})),
        ("test_tool".to_string(), serde_json::json!({"value": 2, "operation": "square"})),
        ("test_tool".to_string(), serde_json::json!({"value": 3, "operation": "increment"})),
    ];
    
    let results = executor.execute_parallel(requests).await;
    assert_eq!(results.len(), 3);
    
    for result in results {
        let exec_result = result.unwrap();
        assert!(exec_result.success);
    }
}

#[tokio::test]
async fn test_registry_features() {
    let registry = ToolRegistry::builder()
        .with_tool(TestTool::new())
        .unwrap()
        .build();
    
    // Test registry queries
    assert_eq!(registry.len(), 1);
    assert!(registry.has_tool("test_tool"));
    assert!(!registry.has_tool("nonexistent"));
    
    let tool_names = registry.tool_names();
    assert_eq!(tool_names.len(), 1);
    assert!(tool_names.contains(&"test_tool".to_string()));
    
    let metadata = registry.metadata("test_tool").unwrap();
    assert_eq!(metadata.name, "test_tool");
    assert_eq!(metadata.version, "1.0.0");
    
    let schema = registry.input_schema("test_tool").unwrap();
    assert!(schema.get("properties").is_some());
}

#[tokio::test]
async fn test_error_context() {
    let error = error_context()
        .with_tool("test_tool")
        .invalid_parameters("Test error message");
    
    let error_str = format!("{}", error);
    assert!(error_str.contains("test_tool"));
    assert!(error_str.contains("Test error message"));
}

#[tokio::test]
async fn test_builtin_calculator() {
    use zai_tools::builtin::CalculatorTool;
    
    let registry = ToolRegistry::builder()
        .with_tool(CalculatorTool::new())
        .unwrap()
        .build();
    
    let executor = ToolExecutor::builder(registry).build();
    
    let input = serde_json::json!({
        "operation": "add",
        "a": 10.0,
        "b": 20.0
    });
    
    let result = executor.execute("calculator", input).await.unwrap();
    assert!(result.success);
    
    let output: serde_json::Value = result.result;
    assert_eq!(output["result"].as_f64().unwrap(), 30.0);
}
