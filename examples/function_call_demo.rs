//! Simple function call demo using zai-tools
//!
//! This example demonstrates basic tool definition and execution
//! without requiring LLM API calls.

use serde::{Deserialize, Serialize};
use zai_tools::prelude::*;

// Weather tool
#[derive(Debug, Clone, Serialize, Deserialize)]
struct WeatherInput {
    city: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WeatherOutput {
    city: String,
    temperature: i32,
    condition: String,
}

// Calculator tool
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CalcInput {
    a: f64,
    b: f64,
    operation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CalcOutput {
    result: f64,
}

impl ToolInput for WeatherInput {
    fn validate(&self) -> ToolResult<()> {
        if self.city.trim().is_empty() {
            return Err(error_context()
                .with_tool("get_weather")
                .invalid_parameters("City name cannot be empty"));
        }
        Ok(())
    }

    fn schema() -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "city": {
                    "type": "string",
                    "description": "City name"
                }
            },
            "required": ["city"]
        })
    }
}

impl ToolOutput for WeatherOutput {}

impl ToolInput for CalcInput {
    fn validate(&self) -> ToolResult<()> {
        match self.operation.as_str() {
            "add" | "subtract" | "multiply" | "divide" => Ok(()),
            _ => Err(error_context()
                .with_tool("calculator")
                .invalid_parameters("Invalid operation")),
        }
    }

    fn schema() -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "a": {"type": "number"},
                "b": {"type": "number"},
                "operation": {
                    "type": "string",
                    "enum": ["add", "subtract", "multiply", "divide"]
                }
            },
            "required": ["a", "b", "operation"]
        })
    }
}

impl ToolOutput for CalcOutput {}

#[derive(Clone)]
struct WeatherTool {
    metadata: ToolMetadata,
}

impl WeatherTool {
    fn new() -> Self {
        Self {
            metadata: ToolMetadata::new::<WeatherInput, WeatherOutput>(
                "get_weather",
                "Get weather for a city"
            ),
        }
    }
}

#[async_trait]
impl Tool<WeatherInput, WeatherOutput> for WeatherTool {
    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }

    async fn execute(&self, input: WeatherInput) -> ToolResult<WeatherOutput> {
        println!("Getting weather for: {}", input.city);

        Ok(WeatherOutput {
            city: input.city,
            temperature: 25,
            condition: "Sunny".to_string(),
        })
    }
}

#[derive(Clone)]
struct CalculatorTool {
    metadata: ToolMetadata,
}

impl CalculatorTool {
    fn new() -> Self {
        Self {
            metadata: ToolMetadata::new::<CalcInput, CalcOutput>(
                "calculator",
                "Perform basic math operations"
            ),
        }
    }
}

#[async_trait]
impl Tool<CalcInput, CalcOutput> for CalculatorTool {
    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }

    async fn execute(&self, input: CalcInput) -> ToolResult<CalcOutput> {
        println!("Calculating: {} {} {}", input.a, input.operation, input.b);

        let result = match input.operation.as_str() {
            "add" => input.a + input.b,
            "subtract" => input.a - input.b,
            "multiply" => input.a * input.b,
            "divide" => {
                if input.b == 0.0 {
                    return Err(error_context()
                        .with_tool("calculator")
                        .invalid_parameters("Division by zero"));
                }
                input.a / input.b
            }
            _ => unreachable!(), // Already validated
        };

        Ok(CalcOutput { result })
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Multi-Tool Function Call Demo");

    // Create tool registry with multiple tools (fluent chaining)
    let registry = ToolRegistry::builder()
        .add_tool(WeatherTool::new())
        .add_tool(CalculatorTool::new())
        .build();

    let executor = ToolExecutor::builder(registry)
        .build();

    println!("📋 Registered tools: {:?}", executor.registry().tool_names());

    // Test weather tool
    println!("\n🌤️ Testing Weather Tool:");
    let cities = vec!["Beijing", "Shanghai", ""];

    for city in cities {
        println!("\n🧪 Testing city: '{}'", city);

        let arguments = serde_json::json!({ "city": city });
        let result = executor.execute("get_weather", arguments).await?;

        if result.success {
            println!("✅ Success: {}", serde_json::to_string_pretty(&result.result)?);
        } else {
            println!("❌ Failed: {}", result.error.unwrap_or_default());
        }
    }

    // Test calculator tool
    println!("\n🧮 Testing Calculator Tool:");
    let calculations = vec![
        ("add", 10.0, 5.0),
        ("multiply", 3.0, 7.0),
        ("divide", 15.0, 3.0),
        ("divide", 10.0, 0.0), // This should fail
    ];

    for (op, a, b) in calculations {
        println!("\n🧪 Testing: {} {} {}", a, op, b);

        let arguments = serde_json::json!({
            "a": a,
            "b": b,
            "operation": op
        });
        let result = executor.execute("calculator", arguments).await?;

        if result.success {
            println!("✅ Success: {}", serde_json::to_string_pretty(&result.result)?);
        } else {
            println!("❌ Failed: {}", result.error.unwrap_or_default());
        }
    }

    println!("\n🎉 Demo completed!");
    Ok(())
}
