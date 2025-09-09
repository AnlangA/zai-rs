# ZAI Tools User Guide

This comprehensive guide will help you get the most out of ZAI Tools, from basic usage to advanced patterns and best practices.

## Table of Contents

1. [Getting Started](#getting-started)
2. [Core Concepts](#core-concepts)
3. [Creating Tools](#creating-tools)
4. [Tool Registry](#tool-registry)
5. [Tool Execution](#tool-execution)
6. [Built-in Tools](#built-in-tools)
7. [Error Handling](#error-handling)
8. [Performance Optimization](#performance-optimization)
9. [Best Practices](#best-practices)
10. [Advanced Patterns](#advanced-patterns)

## Getting Started

### Installation

Add ZAI Tools to your `Cargo.toml`:

```toml
[dependencies]
zai-tools = "2.0.0"
tokio = { version = "1.0", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

### Your First Tool

```rust
use zai_tools::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GreetInput {
    name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GreetOutput {
    message: String,
}

impl ToolInput for GreetInput {}
impl ToolOutput for GreetOutput {}

#[derive(Clone)]
struct GreetTool {
    metadata: ToolMetadata,
}

impl GreetTool {
    fn new() -> Self {
        Self {
            metadata: ToolMetadata::new::<GreetInput, GreetOutput>(
                "greet",
                "A friendly greeting tool"
            ),
        }
    }
}

#[async_trait]
impl Tool<GreetInput, GreetOutput> for GreetTool {
    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }
    
    async fn execute(&self, input: GreetInput) -> ToolResult<GreetOutput> {
        Ok(GreetOutput {
            message: format!("Hello, {}!", input.name),
        })
    }
}
```

## Core Concepts

### Tools

A **Tool** is a unit of functionality that can be executed by AI models. Each tool has:

- **Input Type**: Strongly-typed parameters (implements `ToolInput`)
- **Output Type**: Strongly-typed result (implements `ToolOutput`)
- **Metadata**: Name, description, version, tags, etc.
- **Execute Method**: Async function that performs the work

### Tool Registry

The **ToolRegistry** is a thread-safe container that manages all available tools. It provides:

- Tool registration and lookup
- Metadata queries
- Schema generation
- Concurrent access support

### Tool Executor

The **ToolExecutor** handles tool execution with:

- Timeout management
- Retry logic
- Parallel execution
- Error handling
- Performance monitoring

## Creating Tools

### Basic Tool Structure

```rust
#[derive(Clone)]
struct MyTool {
    metadata: ToolMetadata,
    // Add any configuration or state here
}

impl MyTool {
    fn new() -> Self {
        Self {
            metadata: ToolMetadata::new::<MyInput, MyOutput>(
                "my_tool",
                "Description of what this tool does"
            )
            .version("1.0.0")
            .author("Your Name")
            .tags(["category", "feature"]),
        }
    }
}

#[async_trait]
impl Tool<MyInput, MyOutput> for MyTool {
    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }
    
    async fn execute(&self, input: MyInput) -> ToolResult<MyOutput> {
        // Your tool logic here
        todo!()
    }
}
```

### Input Validation

Implement custom validation for your input types:

```rust
impl ToolInput for MyInput {
    fn validate(&self) -> ToolResult<()> {
        if self.value < 0 {
            return Err(error_context()
                .invalid_parameters("Value must be non-negative"));
        }
        
        if self.name.is_empty() {
            return Err(error_context()
                .invalid_parameters("Name cannot be empty"));
        }
        
        Ok(())
    }
    
    fn schema() -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "value": {
                    "type": "integer",
                    "minimum": 0,
                    "description": "A non-negative integer"
                },
                "name": {
                    "type": "string",
                    "minLength": 1,
                    "description": "A non-empty name"
                }
            },
            "required": ["value", "name"]
        })
    }
}
```

### Async Operations

Tools can perform async operations like HTTP requests or file I/O:

```rust
#[async_trait]
impl Tool<HttpInput, HttpOutput> for HttpTool {
    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }
    
    async fn execute(&self, input: HttpInput) -> ToolResult<HttpOutput> {
        let client = reqwest::Client::new();
        
        let response = client
            .get(&input.url)
            .timeout(std::time::Duration::from_secs(30))
            .send()
            .await
            .map_err(|e| error_context()
                .with_tool("http_tool")
                .execution_failed(format!("HTTP request failed: {}", e)))?;
        
        let text = response
            .text()
            .await
            .map_err(|e| error_context()
                .with_tool("http_tool")
                .execution_failed(format!("Failed to read response: {}", e)))?;
        
        Ok(HttpOutput {
            status: response.status().as_u16(),
            body: text,
        })
    }
}
```

## Tool Registry

### Creating a Registry

```rust
// Empty registry
let registry = ToolRegistry::new();

// Registry with builder pattern - Method 1: Fluent chaining
let registry = ToolRegistry::builder()
    .add_tool(CalculatorTool::new())
    .add_tool(HttpTool::new())
    .add_tool(FileToolTool::new())
    .build();

// Method 2: With error handling
let registry = ToolRegistry::builder()
    .with_tool(CalculatorTool::new())?
    .with_tool(HttpTool::new())?
    .with_tool(FileToolTool::new())?
    .build();

// Method 3: Try chaining (ignores errors)
let registry = ToolRegistry::builder()
    .try_add_tool(CalculatorTool::new())
    .try_add_tool(HttpTool::new())
    .try_add_tool(FileToolTool::new())
    .build();

// Registry with built-in tools
let registry = ToolRegistry::new();
zai_tools::builtin::register_all_builtin_tools(&registry)?;
```

### Registry Operations

```rust
// Check if tool exists
if registry.has_tool("calculator") {
    println!("Calculator tool is available");
}

// Get tool metadata
if let Some(metadata) = registry.metadata("calculator") {
    println!("Tool: {} v{}", metadata.name, metadata.version);
    println!("Description: {}", metadata.description);
    println!("Tags: {:?}", metadata.tags);
}

// Get input schema
if let Some(schema) = registry.input_schema("calculator") {
    println!("Schema: {}", serde_json::to_string_pretty(&schema)?);
}

// List all tools
let tool_names = registry.tool_names();
println!("Available tools: {:?}", tool_names);

// Get registry statistics
println!("Total tools: {}", registry.len());
```

## Tool Execution

### Basic Execution

```rust
let executor = ToolExecutor::builder(registry)
    .timeout(std::time::Duration::from_secs(30))
    .retries(3)
    .logging(true)
    .build();

let input = serde_json::json!({
    "operation": "add",
    "a": 10.0,
    "b": 20.0
});

let result = executor.execute("calculator", input).await?;

if result.success {
    println!("Result: {}", serde_json::to_string_pretty(&result.result)?);
    println!("Execution time: {:?}", result.duration);
} else {
    eprintln!("Error: {}", result.error.unwrap_or_default());
}
```

### Parallel Execution

```rust
let requests = vec![
    ("calculator".to_string(), serde_json::json!({"operation": "add", "a": 1, "b": 2})),
    ("calculator".to_string(), serde_json::json!({"operation": "multiply", "a": 3, "b": 4})),
    ("calculator".to_string(), serde_json::json!({"operation": "divide", "a": 10, "b": 2})),
];

let results = executor.execute_parallel(requests).await;

for (i, result) in results.iter().enumerate() {
    match result {
        Ok(exec_result) => {
            if exec_result.success {
                println!("Task {}: Success in {:?}", i + 1, exec_result.duration);
            } else {
                println!("Task {}: Failed - {}", i + 1, exec_result.error.as_ref().unwrap());
            }
        }
        Err(e) => {
            println!("Task {}: Error - {}", i + 1, e);
        }
    }
}
```

### Simple Execution (for quick testing)

```rust
// For simple cases where you don't need detailed execution info
let result_value = executor.execute_simple("calculator", input).await?;
println!("Result: {}", result_value);
```

## Built-in Tools

ZAI Tools comes with several built-in tools:

### Calculator Tool

```rust
use zai_tools::builtin::CalculatorTool;

let registry = ToolRegistry::builder()
    .with_tool(CalculatorTool::new())?
    .build();

// Supports: add, subtract, multiply, divide, power, sqrt, abs
let input = serde_json::json!({
    "operation": "sqrt",
    "a": 16.0,
    "b": 0.0  // Not used for sqrt
});
```

### Text Processor Tool

```rust
use zai_tools::builtin::TextProcessorTool;

// Supports: uppercase, lowercase, reverse, length, word_count, etc.
let input = serde_json::json!({
    "operation": "uppercase",
    "text": "hello world"
});
```

### JSON Tool

```rust
use zai_tools::builtin::JsonTool;

// Supports: validate, format, minify, extract, etc.
let input = serde_json::json!({
    "operation": "format",
    "data": {"name": "John", "age": 30}
});
```

## Error Handling

### Error Types

ZAI Tools provides comprehensive error handling:

```rust
use zai_tools::error::{ToolError, ToolResult};

match result {
    Ok(value) => println!("Success: {}", value),
    Err(ToolError::ToolNotFound { name }) => {
        eprintln!("Tool '{}' not found", name);
    }
    Err(ToolError::InvalidParameters { tool, message }) => {
        eprintln!("Invalid parameters for '{}': {}", tool, message);
    }
    Err(ToolError::ExecutionFailed { tool, message }) => {
        eprintln!("Execution failed for '{}': {}", tool, message);
    }
    Err(ToolError::TimeoutError { tool, timeout }) => {
        eprintln!("Tool '{}' timed out after {:?}", tool, timeout);
    }
    Err(e) => {
        eprintln!("Other error: {}", e);
    }
}
```

### Error Context

Use error context for better error messages:

```rust
// In your tool implementation
if some_condition {
    return Err(error_context()
        .with_tool("my_tool")
        .execution_failed("Detailed error message"));
}

// For validation errors
if input.value < 0 {
    return Err(error_context()
        .invalid_parameters("Value must be non-negative"));
}
```

## Performance Optimization

### Registry Performance

- Use `ToolRegistry::builder()` for batch registration
- Consider using `Arc<ToolRegistry>` for sharing across threads
- Registry lookups are O(1) with HashMap-based storage

### Execution Performance

- Use parallel execution for independent operations
- Set appropriate timeouts to avoid hanging
- Consider tool-specific optimizations (caching, connection pooling)

### Memory Usage

- Tools are stored as trait objects with minimal overhead
- Input/output serialization is optimized with serde
- Registry uses efficient data structures

## Best Practices

### Tool Design

1. **Keep tools focused**: Each tool should have a single, well-defined purpose
2. **Use strong typing**: Leverage Rust's type system for safety
3. **Validate inputs**: Always validate input parameters
4. **Handle errors gracefully**: Provide meaningful error messages
5. **Document thoroughly**: Use clear descriptions and examples

### Error Handling

1. **Use specific error types**: Don't use generic error messages
2. **Provide context**: Include tool name and operation details
3. **Log appropriately**: Use structured logging for debugging
4. **Fail fast**: Validate inputs before expensive operations

### Performance

1. **Use async/await**: Don't block the executor thread
2. **Set timeouts**: Prevent tools from hanging indefinitely
3. **Cache when appropriate**: Avoid repeated expensive operations
4. **Monitor performance**: Use execution metrics for optimization

### Testing

1. **Unit test tools**: Test each tool in isolation
2. **Integration test workflows**: Test tool combinations
3. **Property-based testing**: Use proptest for edge cases
4. **Benchmark performance**: Use criterion for performance testing

## Advanced Patterns

### Tool Composition

```rust
// Create a composite tool that uses other tools
#[derive(Clone)]
struct CompositeWorkflow {
    registry: Arc<ToolRegistry>,
    executor: Arc<ToolExecutor>,
    metadata: ToolMetadata,
}

#[async_trait]
impl Tool<WorkflowInput, WorkflowOutput> for CompositeWorkflow {
    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }
    
    async fn execute(&self, input: WorkflowInput) -> ToolResult<WorkflowOutput> {
        // Step 1: Process text
        let text_result = self.executor.execute_simple(
            "text_processor",
            serde_json::json!({"operation": "uppercase", "text": input.text})
        ).await?;
        
        // Step 2: Calculate something
        let calc_result = self.executor.execute_simple(
            "calculator",
            serde_json::json!({"operation": "multiply", "a": input.factor, "b": 2.0})
        ).await?;
        
        // Combine results
        Ok(WorkflowOutput {
            processed_text: text_result.as_str().unwrap().to_string(),
            calculated_value: calc_result["result"].as_f64().unwrap(),
        })
    }
}
```

### Dynamic Tool Loading

```rust
// Load tools dynamically based on configuration
fn load_tools_from_config(config: &Config) -> ToolResult<ToolRegistry> {
    let mut builder = ToolRegistry::builder();
    
    for tool_config in &config.tools {
        match tool_config.name.as_str() {
            "calculator" => builder = builder.with_tool(CalculatorTool::new())?,
            "http" => builder = builder.with_tool(HttpTool::new())?,
            "file" => builder = builder.with_tool(FileTool::new())?,
            _ => return Err(error_context()
                .invalid_parameters(format!("Unknown tool: {}", tool_config.name))),
        }
    }
    
    Ok(builder.build())
}
```

### Tool Middleware

```rust
// Implement logging middleware
#[derive(Clone)]
struct LoggingTool<T> {
    inner: T,
    logger: Arc<dyn Logger>,
}

#[async_trait]
impl<T, I, O> Tool<I, O> for LoggingTool<T>
where
    T: Tool<I, O> + Clone,
    I: ToolInput,
    O: ToolOutput,
{
    fn metadata(&self) -> &ToolMetadata {
        self.inner.metadata()
    }
    
    async fn execute(&self, input: I) -> ToolResult<O> {
        let start = std::time::Instant::now();
        let tool_name = self.metadata().name.clone();
        
        self.logger.info(&format!("Executing tool: {}", tool_name));
        
        let result = self.inner.execute(input).await;
        let duration = start.elapsed();
        
        match &result {
            Ok(_) => self.logger.info(&format!(
                "Tool {} completed successfully in {:?}", 
                tool_name, duration
            )),
            Err(e) => self.logger.error(&format!(
                "Tool {} failed after {:?}: {}", 
                tool_name, duration, e
            )),
        }
        
        result
    }
}
```

This guide covers the essential concepts and patterns for using ZAI Tools effectively. For more examples and advanced use cases, check out the examples directory and API documentation.
