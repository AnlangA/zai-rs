//! Function calling with LLM using zai-tools
//!
//! This example shows how to integrate zai-tools with LLM function calling.

use serde_json::json;
use zai_rs::{
    model::{chat_base_response::ChatCompletionResponse, *},
    toolkits::prelude::*,
};

fn make_weather_tool() -> FunctionTool {
    FunctionTool::builder("get_weather", "Get weather for a city")
        .schema(json!({
            "type": "object",
            "properties": { "city": { "type": "string", "description": "City name" } },
            "required": ["city"],
            "additionalProperties": false
        }))
        .handler(|args| async move {
            // 获取参数
            let city = args
                .get("city")
                .and_then(|v| v.as_str())
                .unwrap_or_else(|| "");

            // 参数验证
            if city.trim().is_empty() {
                return Err(error_context()
                    .with_tool("get_weather")
                    .with_operation("参数验证")
                    .invalid_parameters("City name cannot be empty"));
            }

            // 模拟天气数据
            let weather = match city.to_lowercase().as_str() {
                "深圳" => json!({
                    "city": "Shenzhen",
                    "temperature": 28,
                    "condition": "Sunny"
                }),
                "beijing" => json!({
                    "city": "Beijing",
                    "temperature": 15,
                    "condition": "Cloudy"
                }),
                "shanghai" => json!({
                    "city": "Shanghai",
                    "temperature": 22,
                    "condition": "Rainy"
                }),
                _ => json!({
                    "city": city,
                    "temperature": 20,
                    "condition": "Unknown"
                }),
            };

            Ok(weather)
        })
        .build()
        .expect("weather tool")
}

fn make_calc_tool() -> FunctionTool {
    FunctionTool::builder("calc", "Simple arithmetic calculation: add/sub/mul/div")
        .property("op", json!({ "type": "string", "enum": ["add", "sub", "mul", "div"], "description": "Operation" }))
        .property("a", json!({ "type": "number", "description": "Left operand" }))
        .property("b", json!({ "type": "number", "description": "Right operand" }))
        .required("op").required("a").required("b")
        .handler(|args| async move {
            let op = args.get("op").and_then(|v| v.as_str()).unwrap_or_else(|| "");

            // 使用 ErrorContext 的 with_operation 来添加上下文信息
            let a = args.get("a").and_then(|v| v.as_f64())
                .ok_or_else(|| error_context()
                    .with_tool("calc")
                    .with_operation("解析左操作数")
                    .invalid_parameters("Missing number 'a'"))?;

            let b = args.get("b").and_then(|v| v.as_f64())
                .ok_or_else(|| error_context()
                    .with_tool("calc")
                    .with_operation("解析右操作数")
                    .invalid_parameters("Missing number 'b'"))?;

            let result = match op {
                "add" => a + b,
                "sub" => a - b,
                "mul" => a * b,
                "div" => {
                    // 演示 with_operation 的用法 - 除零检查
                    if b == 0.0 {
                        return Err(error_context()
                            .with_tool("calc")
                            .with_operation("除法运算")
                            .invalid_parameters("Division by zero"));
                    }
                    a / b
                },
                _ => return Err(error_context()
                    .with_tool("calc")
                    .with_operation("操作符验证")
                    .invalid_parameters("Unsupported op, expected one of add/sub/mul/div")),
            };
            Ok(json!({ "op": op, "a": a, "b": b, "result": result }))
        })
        .build()
        .expect("calc tool")
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    // Setup tools (executor owns its registry)
    let executor = ToolExecutor::new();
    executor
        .add_dyn_tool(Box::new(make_weather_tool()))
        .add_dyn_tool(Box::new(make_calc_tool()));

    // Create LLM function definitions (both tools)
    let tool_defs = executor.export_all_tools_as_functions();

    // Setup LLM client
    let key = get_key()?;
    let user_text = "帮我查找深圳今天的天气，然后计算 7 和 5 的加法";

    let mut client = ChatCompletion::new(model(), TextMessage::user(user_text), key)
        .with_thinking(ThinkingType::Disabled)
        .add_tools(tool_defs)
        .with_max_tokens(512);

    // First round
    let last_resp: ChatCompletionResponse = client.send().await?;
    println!("📨 LLM Response: {:#?}", last_resp);

    if let Some(calls) = last_resp
        .choices()
        .and_then(|v| v.first())
        .and_then(|c| c.message().tool_calls())
    {
        let tool_msgs = executor.execute_tool_calls_parallel(calls).await;
        for msg in tool_msgs {
            client = client.add_messages(msg);
        }
        // Remove tools to avoid repeated calls, and nudge model to answer
        client.body_mut().tools = None;
        let sys =
            TextMessage::system("请基于上述工具结果，用中文直接回答用户问题，不要再次调用工具。");
        client = client.add_messages(sys);

        let next_body: ChatCompletionResponse = client.send().await?;
        println!("Model after tool: {:#?}", next_body);
    }

    Ok(())
}

fn model() -> GLM4_5_flash {
    GLM4_5_flash {}
}

fn get_key() -> Result<String, Box<dyn std::error::Error>> {
    match std::env::var("ZHIPU_API_KEY") {
        Ok(key) => Ok(key),
        Err(_) => {
            println!("Please enter your ZHIPU_API_KEY:");
            let mut key = String::new();
            std::io::stdin().read_line(&mut key)?;
            Ok(key.trim().to_string())
        },
    }
}
