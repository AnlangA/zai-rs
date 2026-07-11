//! # Basic Chat Text Example
//!
//! Demonstrates basic text chat completion via `ZaiClient` (P05 migration).

use zai_rs::client::v2::ZaiClient;
use zai_rs::model::{chat_base_response::ChatCompletionResponse, *};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let model = GLM4_5_flash {};
    let client = ZaiClient::from_env()?;

    let user_text = "你好";

    let request = ChatCompletion::new(model, TextMessage::user(user_text))
        .with_temperature(0.7)
        .with_top_p(0.9)
        .with_thinking(ThinkingType::disabled());

    let body: ChatCompletionResponse = request.send_via(&client).await?;
    println!("{body:#?}");

    Ok(())
}
