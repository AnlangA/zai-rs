# ZAI Tools

[![Crates.io](https://img.shields.io/crates/v/zai-tools.svg)](https://crates.io/crates/zai-tools)
[![Documentation](https://docs.rs/zai-tools/badge.svg)](https://docs.rs/zai-tools)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)
[![Build Status](https://github.com/AnlangA/zai-rs/workflows/CI/badge.svg)](https://github.com/AnlangA/zai-rs/actions)

🚀 **A powerful and flexible tool system for AI function calling with enhanced type safety and performance.**

ZAI Tools provides a comprehensive framework for defining, registering, and executing tools that can be called by AI models, with a focus on type safety, performance, and developer experience.

## ✨ Features

- 🔒 **Type-safe tool definitions** with automatic schema generation and validation
- ⚡ **High-performance async execution** with parallel processing and timeout support
- 🛠️ **Rich built-in tools** for common use cases (HTTP, file operations, calculations, text processing)
- 🔧 **Flexible plugin system** with dynamic tool loading and dependency injection
- 📊 **Enterprise-ready** with comprehensive logging, monitoring, and configuration management
- 🎯 **Integrated macros** for rapid tool development (no separate crate needed)
- 🧪 **Comprehensive testing** with unit tests, integration tests, and benchmarks
- 📚 **Excellent documentation** with examples, tutorials, and best practices
- 🔒 **Thread-safe** registry and executor with concurrent access support
- 🚀 **Performance optimized** with efficient memory usage and minimal overhead

## 🚀 Quick Start

Add this to your `Cargo.toml`:

```toml
[dependencies]
zai-tools = "2.0.0"
tokio = { version = "1.0", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

### 🎯 Type-Safe API (Recommended)

The main API provides full type safety and excellent performance:

```rust
use zai_tools::prelude::*;
use serde::{Deserialize, Serialize};

// Define strongly-typed input and output
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CalculatorInput {
    operation: String,
    a: f64,
    b: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CalculatorOutput {
    result: f64,
    expression: String,
}

// Implement the required traits
impl ToolInput for CalculatorInput {
    fn validate(&self) -> ToolResult<()> {
        match self.operation.as_str() {
            "add" | "subtract" | "multiply" | "divide" => Ok(()),
            _ => Err(error_context().invalid_parameters("Invalid operation")),
        }
    }
}

impl ToolOutput for CalculatorOutput {}

// Define your tool
#[derive(Clone)]
struct Calculator {
    metadata: ToolMetadata,
}

impl Calculator {
    fn new() -> Self {
        Self {
            metadata: ToolMetadata::new::<CalculatorInput, CalculatorOutput>(
                "calculator",
                "A type-safe calculator tool"
            )
            .version("1.0.0")
            .tags(["math", "calculator"]),
        }
    }
}

#[async_trait]
impl Tool<CalculatorInput, CalculatorOutput> for Calculator {
    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }

    async fn execute(&self, input: CalculatorInput) -> ToolResult<CalculatorOutput> {
        let result = match input.operation.as_str() {
            "add" => input.a + input.b,
            "subtract" => input.a - input.b,
            "multiply" => input.a * input.b,
            "divide" => {
                if input.b == 0.0 {
                    return Err(error_context()
                        .with_tool("calculator")
                        .execution_failed("Division by zero"));
                }
                input.a / input.b
            }
            _ => unreachable!(), // Validation catches this
        };

        Ok(CalculatorOutput {
            result,
            expression: format!("{} {} {} = {}", input.a, input.operation, input.b, result),
        })
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create registry and register tools (fluent chaining)
    let registry = ToolRegistry::builder()
        .add_tool(Calculator::new())
        .add_tool(WeatherTool::new())
        .add_tool(StringTool::new())
        .build();

    // Alternative: with error handling
    let registry_alt = ToolRegistry::builder()
        .with_tool(Calculator::new())?
        .with_tool(WeatherTool::new())?
        .build();

    // Create executor with configuration
    let executor = ToolExecutor::builder(registry)
        .timeout(std::time::Duration::from_secs(30))
        .retries(3)
        .logging(true)
        .build();

    // Execute with type-safe input
    let input = serde_json::json!({
        "operation": "multiply",
        "a": 15.5,
        "b": 24.3
    });

    let result = executor.execute("calculator", input).await?;
    println!("Result: {}", serde_json::to_string_pretty(&result.result)?);

    Ok(())
}
```

### 🎯 Using Integrated Macros

ZAI Tools now includes declarative macros directly in the main crate (no separate zai-tools-macros needed):

```rust
use zai_tools::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MathInput { a: f64, b: f64 }

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MathOutput { result: f64 }

impl ToolInput for MathInput {}
impl ToolOutput for MathOutput {}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create tools using integrated macros
    let add_tool = simple_tool! {
        name: "add",
        description: "Add two numbers",
        input: MathInput,
        output: MathOutput,
        execute: |input: MathInput| -> ToolResult<MathOutput> {
            Ok(MathOutput { result: input.a + input.b })
        }
    };

    let registry = zai_tools::macros::registry()
        .with_tool(add_tool)?
        .build();

    let executor = ToolExecutor::builder(registry).build();

    let result = executor.execute("add", serde_json::json!({
        "a": 10.0, "b": 20.0
    })).await?;

    println!("Result: {}", serde_json::to_string_pretty(&result.result)?);
    Ok(())
}
```

### 🔧 Multi-Tool Registration

ZAI Tools supports multiple ways to register tools with fluent chaining:

```rust
// Method 1: Fluent chaining (panics on error)
let registry = ToolRegistry::builder()
    .add_tool(CalculatorTool::new())
    .add_tool(WeatherTool::new())
    .add_tool(StringTool::new())
    .build();

// Method 2: With error handling (returns Result)
let registry = ToolRegistry::builder()
    .with_tool(CalculatorTool::new())?
    .with_tool(WeatherTool::new())?
    .with_tool(StringTool::new())?
    .build();

// Method 3: Try chaining (ignores errors)
let registry = ToolRegistry::builder()
    .try_add_tool(CalculatorTool::new())
    .try_add_tool(WeatherTool::new())
    .try_add_tool(StringTool::new())
    .build();

// Check registered tools
println!("Registered tools: {:?}", registry.tool_names());
```

### 🎯 Easy API

Great balance of simplicity and power:

```rust
use zai_tools::easy::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let tools = Tools::new()
        .add_simple("double", "Double a number", |x: f64| x * 2.0)
        .add_async("fetch", "Fetch data", |id: u32| async move {
            format!("Data for ID: {}", id)
        });

    let result = tools.run("double", 21.0).await?;
    println!("Double 21 = {}", result); // 42.0

    Ok(())
}
```

### 🔒 Type-Safe API (V2)

For production applications requiring full type safety:

```rust
use zai_tools::v2::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct Input { name: String }

#[derive(Serialize)]
struct Output { greeting: String }

impl ToolInput for Input {}
impl ToolOutput for Output {}

#[derive(Clone)]
struct GreetTool { metadata: ToolMetadata }

#[async_trait]
impl Tool<Input, Output> for GreetTool {
    fn metadata(&self) -> &ToolMetadata { &self.metadata }

    async fn execute(&self, input: Input) -> ToolResult<Output> {
        Ok(Output { greeting: format!("Hello, {}!", input.name) })
    }
}
```

### Creating Custom Tools

```rust
use async_trait::async_trait;
use serde_json::Value;
use zai_tools::prelude::*;

struct MyCustomTool {
    metadata: ToolMetadata,
}

impl MyCustomTool {
    fn new() -> Self {
        let schema = SchemaBuilder::new()
            .add_string("input", "Input text to process", true)
            .build();
        
        let metadata = ToolMetadata::new(
            "my_tool",
            "A custom tool that processes text"
        ).with_parameters_schema(schema);
        
        Self { metadata }
    }
}

#[async_trait]
impl Tool for MyCustomTool {
    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }
    
    async fn execute(&self, parameters: Value) -> ToolResult<Value> {
        let input = parameters["input"].as_str()
            .ok_or_else(|| ToolError::invalid_parameters("Missing input"))?;
        
        // Process the input
        let result = format!("Processed: {}", input);
        
        Ok(serde_json::json!({"result": result}))
    }
}

// Register and use the custom tool
let registry = ToolRegistry::new();
registry.register(MyCustomTool::new())?;
```

## Built-in Tools

ZAI Tools comes with several built-in tools:

### Weather Tool (`get_weather`)
Get weather information for a city.

```rust
let result = executor.execute_simple(
    "get_weather",
    serde_json::json!({"city": "Shanghai"})
).await?;
```

### Calculator Tool (`calculate`)
Perform basic mathematical operations.

```rust
let result = executor.execute_simple(
    "calculate",
    serde_json::json!({
        "operation": "multiply",
        "a": 15.5,
        "b": 24.3
    })
).await?;
```

### Text Processing Tool (`process_text`)
Process text with various operations.

```rust
let result = executor.execute_simple(
    "process_text",
    serde_json::json!({
        "operation": "uppercase",
        "text": "hello world"
    })
).await?;
```

### Time Utilities Tool (`time_utils`)
Perform time and date operations.

```rust
let result = executor.execute_simple(
    "time_utils",
    serde_json::json!({
        "operation": "now",
        "format": "%Y-%m-%d %H:%M:%S"
    })
).await?;
```

## Advanced Features

### Parallel Execution

```rust
let requests = vec![
    ("get_weather".to_string(), serde_json::json!({"city": "Beijing"})),
    ("calculate".to_string(), serde_json::json!({"operation": "add", "a": 10, "b": 20})),
];

let results = executor.execute_parallel(requests).await;
```

### Execution Configuration

```rust
let config = ExecutionConfig::new()
    .with_timeout(Duration::from_secs(10))
    .with_retries(3)
    .with_logging(true);

let executor = ToolExecutor::with_config(registry, config);
```

### Error Handling

```rust
match executor.execute("my_tool", params).await {
    Ok(result) => {
        if result.success {
            println!("Success: {}", serde_json::to_string_pretty(&result.result)?);
        } else {
            println!("Tool failed: {}", result.error.unwrap_or_default());
        }
    }
    Err(e) => {
        match e.kind() {
            ToolErrorKind::NotFound => println!("Tool not found"),
            ToolErrorKind::InvalidParameters => println!("Invalid parameters"),
            ToolErrorKind::ExecutionFailed => println!("Execution failed"),
            _ => println!("Other error: {}", e),
        }
    }
}
```

## Architecture

ZAI Tools is built with a modular architecture:

- **Core**: Defines the `Tool` trait and basic types
- **Registry**: Thread-safe tool registration and discovery
- **Executor**: Advanced execution engine with retry, timeout, and parallel execution
- **Schema**: JSON schema generation and validation utilities
- **Built-in**: Common tools for everyday use

## License

This project is licensed under the MIT OR Apache-2.0 license.
