//! Send one non-streaming text chat request.

use zai_rs::{
    client::ZaiClient,
    model::{
        GLM4_5_flash, TextMessage, ThinkingType, chat::ChatCompletion,
        chat_base_response::ChatCompletionResponse,
    },
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let prompt = std::env::args().nth(1).unwrap_or_else(|| "你好".to_owned());

    let client = ZaiClient::from_env()?;
    let request = ChatCompletion::new(GLM4_5_flash {}, TextMessage::user(prompt))
        .with_temperature(0.7)
        .with_top_p(0.9)
        .with_thinking(ThinkingType::disabled());

    let body: ChatCompletionResponse = request.send_via(&client).await?;
    println!("{body:#?}");

    Ok(())
}
