//! Complete one typed function-call round trip with a deterministic local tool.

use zai_rs::{
    client::ZaiClient,
    model::{
        Function, FunctionParams, GLM4_5_flash, TextMessage, ThinkingType, ToolCall, ToolChoice,
        Tools, chat::ChatCompletion, chat_base_response::ChatCompletionResponse,
    },
};

#[derive(serde::Deserialize)]
struct WeatherArgs {
    city: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let weather = Function::new(
        "get_weather",
        "Return the example weather observation for a city",
        serde_json::json!({
            "type": "object",
            "properties": { "city": { "type": "string" } },
            "required": ["city"],
            "additionalProperties": false
        }),
    );

    let client = ZaiClient::from_env()?;
    let mut request =
        ChatCompletion::new(GLM4_5_flash {}, TextMessage::user("深圳现在的天气怎么样？"))
            .with_thinking(ThinkingType::disabled())
            .add_tool(Tools::Function { function: weather })
            .with_tool_choice(ToolChoice::auto());

    let first: ChatCompletionResponse = request.send_via(&client).await?;
    let (id, name, arguments, assistant_content) = {
        let message = first
            .choices()
            .and_then(|choices| choices.first())
            .ok_or("chat response did not contain a choice")?
            .message()
            .ok_or("chat choice omitted its message")?;
        let call = message
            .tool_calls()
            .and_then(|calls| calls.first())
            .ok_or("model did not return a function call")?;
        let function = call.function().ok_or("tool call omitted its function")?;

        (
            call.id().ok_or("tool call omitted its id")?.to_owned(),
            function.name().to_owned(),
            function.arguments().to_owned(),
            message.content_str().map(str::to_owned),
        )
    };

    if name != "get_weather" {
        return Err(format!("unexpected function call: {name}").into());
    }
    let args: WeatherArgs = serde_json::from_str(&arguments)?;
    let result = serde_json::json!({
        "city": args.city,
        "condition": "晴",
        "temperature_c": 28,
        "source": "example fixture"
    });

    let assistant_call = ToolCall::new_function(&id, FunctionParams::new(name, arguments));
    request = request
        .add_message(TextMessage::assistant_with_tools(
            assistant_content,
            vec![assistant_call],
        ))
        .add_message(TextMessage::tool_with_id(result.to_string(), id))
        .clear_tools();

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
