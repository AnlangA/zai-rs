//! Multi-tool registry demonstration
//! 
//! This example shows how to register multiple tools with fluent chaining
//! and demonstrates different registration patterns.

use serde::{Deserialize, Serialize};
use zai_tools::prelude::*;

// Math tool
#[derive(Debug, Clone, Serialize, Deserialize)]
struct MathInput {
    a: f64,
    b: f64,
    operation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MathOutput {
    result: f64,
}

impl ToolInput for MathInput {
    fn validate(&self) -> ToolResult<()> {
        match self.operation.as_str() {
            "add" | "subtract" | "multiply" | "divide" => Ok(()),
            _ => Err(error_context().invalid_parameters("Invalid operation")),
        }
    }
    
    fn schema() -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "a": {"type": "number"},
                "b": {"type": "number"},
                "operation": {"type": "string", "enum": ["add", "subtract", "multiply", "divide"]}
            },
            "required": ["a", "b", "operation"]
        })
    }
}

impl ToolOutput for MathOutput {}

// String tool
#[derive(Debug, Clone, Serialize, Deserialize)]
struct StringInput {
    text: String,
    operation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StringOutput {
    result: String,
}

impl ToolInput for StringInput {
    fn validate(&self) -> ToolResult<()> {
        match self.operation.as_str() {
            "uppercase" | "lowercase" | "reverse" | "length" => Ok(()),
            _ => Err(error_context().invalid_parameters("Invalid string operation")),
        }
    }
    
    fn schema() -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "text": {"type": "string"},
                "operation": {"type": "string", "enum": ["uppercase", "lowercase", "reverse", "length"]}
            },
            "required": ["text", "operation"]
        })
    }
}

impl ToolOutput for StringOutput {}

// Time tool
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TimeInput {
    format: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TimeOutput {
    timestamp: String,
    formatted: String,
}

impl ToolInput for TimeInput {
    fn validate(&self) -> ToolResult<()> {
        Ok(()) // Always valid
    }
    
    fn schema() -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "format": {"type": "string", "description": "Optional time format"}
            }
        })
    }
}

impl ToolOutput for TimeOutput {}

// Tool implementations
#[derive(Clone)]
struct MathTool {
    metadata: ToolMetadata,
}

impl MathTool {
    fn new() -> Self {
        Self {
            metadata: ToolMetadata::new::<MathInput, MathOutput>("math", "Math operations"),
        }
    }
}

#[async_trait]
impl Tool<MathInput, MathOutput> for MathTool {
    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }
    
    async fn execute(&self, input: MathInput) -> ToolResult<MathOutput> {
        let result = match input.operation.as_str() {
            "add" => input.a + input.b,
            "subtract" => input.a - input.b,
            "multiply" => input.a * input.b,
            "divide" => {
                if input.b == 0.0 {
                    return Err(error_context().invalid_parameters("Division by zero"));
                }
                input.a / input.b
            }
            _ => unreachable!(),
        };
        
        Ok(MathOutput { result })
    }
}

#[derive(Clone)]
struct StringTool {
    metadata: ToolMetadata,
}

impl StringTool {
    fn new() -> Self {
        Self {
            metadata: ToolMetadata::new::<StringInput, StringOutput>("string", "String operations"),
        }
    }
}

#[async_trait]
impl Tool<StringInput, StringOutput> for StringTool {
    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }
    
    async fn execute(&self, input: StringInput) -> ToolResult<StringOutput> {
        let result = match input.operation.as_str() {
            "uppercase" => input.text.to_uppercase(),
            "lowercase" => input.text.to_lowercase(),
            "reverse" => input.text.chars().rev().collect(),
            "length" => input.text.len().to_string(),
            _ => unreachable!(),
        };
        
        Ok(StringOutput { result })
    }
}

#[derive(Clone)]
struct TimeTool {
    metadata: ToolMetadata,
}

impl TimeTool {
    fn new() -> Self {
        Self {
            metadata: ToolMetadata::new::<TimeInput, TimeOutput>("time", "Get current time"),
        }
    }
}

#[async_trait]
impl Tool<TimeInput, TimeOutput> for TimeTool {
    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }
    
    async fn execute(&self, input: TimeInput) -> ToolResult<TimeOutput> {
        use std::time::{SystemTime, UNIX_EPOCH};
        
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let formatted = match input.format.as_deref() {
            Some("iso") => format!("ISO-8601: {}", timestamp),
            Some("unix") => format!("Unix: {}", timestamp),
            _ => format!("Default: {}", timestamp),
        };
        
        Ok(TimeOutput {
            timestamp: timestamp.to_string(),
            formatted,
        })
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Multi-Tool Registry Demo");
    
    // Method 1: Fluent chaining with add_tool (panics on error)
    println!("\n📋 Method 1: Fluent chaining");
    let registry1 = ToolRegistry::builder()
        .add_tool(MathTool::new())
        .add_tool(StringTool::new())
        .add_tool(TimeTool::new())
        .build();
    
    println!("Registered tools: {:?}", registry1.tool_names());
    
    // Method 2: Error handling with with_tool
    println!("\n📋 Method 2: With error handling");
    let registry2 = ToolRegistry::builder()
        .with_tool(MathTool::new())?
        .with_tool(StringTool::new())?
        .with_tool(TimeTool::new())?
        .build();
    
    println!("Registered tools: {:?}", registry2.tool_names());
    
    // Method 3: Try chaining (ignores errors)
    println!("\n📋 Method 3: Try chaining (ignores errors)");
    let registry3 = ToolRegistry::builder()
        .try_add_tool(MathTool::new())
        .try_add_tool(StringTool::new())
        .try_add_tool(TimeTool::new())
        .build();
    
    println!("Registered tools: {:?}", registry3.tool_names());
    
    // Use the first registry for testing
    let executor = ToolExecutor::builder(registry1).build();
    
    // Test all tools
    println!("\n🧮 Testing Math Tool:");
    let math_result = executor.execute("math", serde_json::json!({
        "a": 15.0,
        "b": 3.0,
        "operation": "multiply"
    })).await?;
    println!("Result: {}", serde_json::to_string_pretty(&math_result.result)?);
    
    println!("\n📝 Testing String Tool:");
    let string_result = executor.execute("string", serde_json::json!({
        "text": "Hello World",
        "operation": "reverse"
    })).await?;
    println!("Result: {}", serde_json::to_string_pretty(&string_result.result)?);
    
    println!("\n⏰ Testing Time Tool:");
    let time_result = executor.execute("time", serde_json::json!({
        "format": "iso"
    })).await?;
    println!("Result: {}", serde_json::to_string_pretty(&time_result.result)?);
    
    println!("\n🎉 All tools working perfectly!");
    Ok(())
}
