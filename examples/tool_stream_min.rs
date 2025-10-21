//! Minimal example: streaming tool calls (tool_stream)
//!
//! This example shows how to enable tool_stream with GLM-4.6 and print incremental
//! tool_call payloads while streaming.
//!
//! Run:
//!   cargo run --example tool_stream_min

use serde_json::json;
use tokio;
use zai_rs::model::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1) Read API key
    let key = std::env::var("ZHIPU_API_KEY").expect("Set ZHIPU_API_KEY in your environment");

    // 2) Define a minimal function tool schema (no real execution here; just to trigger tool calls)
    let add_tool = Tools::Function {
        function: Function::new(
            "add",
            "Add two numbers",
            json!({
                "type": "object",
                "properties": {
                    "a": {"type": "number", "description": "left operand"},
                    "b": {"type": "number", "description": "right operand"}
                },
                "required": ["a", "b"],
                "additionalProperties": false
            }),
        ),
    };

    // 3) Build a streaming chat request with tool_stream enabled (GLM-4.6 only)
    let model = GLM4_6 {};
    let mut client = ChatCompletion::new(
        model,
        TextMessage::user("请使用函数 add 计算 7 和 5 的和，然后给出最终答案。"),
        key,
    )
    .add_tools(vec![add_tool])
    .with_coding_plan()
    .enable_stream()
    .with_tool_stream(true);

    client
        .stream_for_each(move |chunk: ChatStreamResponse| async move {
            chunk
                .choices
                .first()
                .and_then(|choice| choice.delta.as_ref())
                .and_then(|delta| delta.tool_calls.as_ref())
                .and_then(|tool_calls| {
                    println!("{:#?}", tool_calls);
                    Some(())
                });
            Ok(())
        })
        .await?;
    Ok(())
}
