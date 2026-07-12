//! Register a toolkit function, execute the model's call, and continue the chat.

use serde_json::json;
use zai_rs::{
    client::ZaiClient,
    model::{
        FunctionParams, GLM4_5_flash, TextMessage, ThinkingType, ToolCall, ToolChoice,
        chat::ChatCompletion,
        chat_base_response::{ChatCompletionResponse, ToolCallMessage},
    },
    toolkits::prelude::{FunctionTool, ToolExecutor, ToolResult, error_context},
};

fn weather_tool() -> ToolResult<FunctionTool> {
    FunctionTool::builder("get_weather", "Return the example weather for a city")
        .schema(json!({
            "type": "object",
            "properties": { "city": { "type": "string" } },
            "required": ["city"],
            "additionalProperties": false
        }))
        .handler(|arguments| async move {
            let city = arguments
                .get("city")
                .and_then(serde_json::Value::as_str)
                .filter(|city| !city.trim().is_empty())
                .ok_or_else(|| {
                    error_context()
                        .with_tool("get_weather")
                        .invalid_parameters("city must be a non-empty string")
                })?;

            Ok(json!({
                "city": city,
                "condition": "晴",
                "temperature_c": 28,
                "source": "example fixture"
            }))
        })
        .build()
}

fn request_tool_call(call: &ToolCallMessage) -> Result<ToolCall, &'static str> {
    let function = call.function().ok_or("tool call omitted its function")?;
    Ok(ToolCall::new_function(
        call.id().ok_or("tool call omitted its id")?,
        FunctionParams::new(function.name(), function.arguments()),
    ))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let executor = ToolExecutor::new();
    executor.add_dyn_tool(Box::new(weather_tool()?))?;

    let client = ZaiClient::from_env()?;
    let mut request = ChatCompletion::new(
        GLM4_5_flash {},
        TextMessage::user("请告诉我深圳现在的天气。"),
    )
    .with_thinking(ThinkingType::disabled())
    .add_tools(executor.export_all_tools_as_functions())
    .with_tool_choice(ToolChoice::auto());

    let first: ChatCompletionResponse = request.send_via(&client).await?;
    let (assistant_message, calls) = {
        let message = first
            .choices()
            .and_then(|choices| choices.first())
            .ok_or("chat response did not contain a choice")?
            .message()
            .ok_or("chat choice omitted its message")?;
        let calls = message
            .tool_calls()
            .filter(|calls| !calls.is_empty())
            .ok_or("model did not return a function call")?
            .to_vec();
        let request_calls = calls
            .iter()
            .map(request_tool_call)
            .collect::<Result<Vec<_>, _>>()?;

        (
            TextMessage::assistant_with_tools(
                message.content_str().map(str::to_owned),
                request_calls,
            ),
            calls,
        )
    };

    request = request.add_message(assistant_message);
    for tool_message in executor.execute_tool_calls_ordered(&calls).await {
        request = request.add_message(tool_message);
    }
    request = request.clear_tools();

    let final_response: ChatCompletionResponse = request.send_via(&client).await?;
    let answer = final_response
        .choices()
        .and_then(|choices| choices.first())
        .and_then(|choice| choice.message())
        .and_then(|message| message.content_str())
        .ok_or("final response did not contain text")?;
    println!("{answer}");

    Ok(())
}
