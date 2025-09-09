//! Function calling with LLM using zai-tools
//!
//! This example shows how to integrate zai-tools with LLM function calling.

use serde::{Deserialize, Serialize};
use zai_rs::client::http::*;
use zai_rs::model::chat::data::ChatCompletion;
use zai_rs::model::*;
use zai_tools::prelude::*;

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
        Ok(WeatherOutput {
            city: input.city,
            temperature: 25,
            condition: "Sunny".to_string(),
        })
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Function Call with LLM Demo");

    // Setup tools
    let registry = ToolRegistry::builder()
        .add_tool(WeatherTool::new())
        .build();

    let executor = ToolExecutor::builder(registry.clone()).build();

    // Create LLM function definition
    let weather_schema = registry.input_schema("get_weather")
        .expect("Weather tool should be registered");

    let weather_func = Function::new("get_weather", "Get weather for a city", weather_schema);
    let tools = Tools::Function { function: weather_func };

    // Setup LLM client
    let key = get_key();
    let user_text = "帮我查找深圳今天的天气";

    let mut client = ChatCompletion::new(model(), TextMessage::user(user_text), key)
        .with_max_tokens(512)
        .with_tools(tools);

    let resp = client.post().await?;
    let v: serde_json::Value = resp.json().await?;
    println!("📨 LLM Response: {}", serde_json::to_string_pretty(&v)?);

    // Handle tool call
    if let Some((id, name, arguments)) = parse_first_tool_call(&v) {
        println!("🔧 Tool call: {} with args: {}", name, arguments);

        let result = execute_tool(&executor, &name, &arguments).await;
        println!("✅ Tool result: {}", serde_json::to_string_pretty(&result)?);

        // Continue conversation
        let tool_msg = TextMessage::tool_with_id(serde_json::to_string(&result)?, id);
        client = client.add_messages(tool_msg);

        let resp2 = client.post().await?;
        let v2: serde_json::Value = resp2.json().await?;
        println!("🔄 Final response: {}", serde_json::to_string_pretty(&v2)?);
    } else {
        println!("❌ No tool calls found");
    }

    Ok(())
}

fn model() -> GLM4_5_flash {
    GLM4_5_flash {}
}

fn get_key() -> String {
    std::env::var("ZHIPU_API_KEY").unwrap_or_else(|_| {
        println!("Please enter your ZHIPU_API_KEY:");
        let mut key = String::new();
        std::io::stdin().read_line(&mut key).unwrap();
        key.trim().to_string()
    })
}

fn parse_first_tool_call(v: &serde_json::Value) -> Option<(String, String, String)> {
    let tool_calls = v.pointer("/choices/0/message/tool_calls")?.as_array()?;
    let tc0 = tool_calls.get(0)?;
    let id = tc0.get("id")?.as_str()?.to_string();
    let func = tc0.get("function")?;
    let name = func.get("name")?.as_str()?.to_string();
    let arguments = func.get("arguments")?.as_str()?.to_string();
    Some((id, name, arguments))
}

async fn execute_tool(
    executor: &ToolExecutor,
    name: &str,
    arguments: &str,
) -> serde_json::Value {
    let args_json: serde_json::Value = match serde_json::from_str(arguments) {
        Ok(v) => v,
        Err(_) => return serde_json::json!({"error": "invalid_arguments"}),
    };

    match executor.execute(name, args_json).await {
        Ok(result) if result.success => result.result,
        Ok(result) => serde_json::json!({"error": result.error.unwrap_or_default()}),
        Err(err) => serde_json::json!({"error": err.to_string()}),
    }
}
