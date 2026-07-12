//! Enable GLM-4.5 thinking and read reasoning separately from the final answer.

use zai_rs::{
    client::ZaiClient,
    model::{
        GLM4_5_air, TextMessage, ThinkingType, chat::ChatCompletion,
        chat_base_response::ChatCompletionResponse,
    },
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = ZaiClient::from_env()?;
    let request = ChatCompletion::new(
        GLM4_5_air {},
        TextMessage::user("为什么系统服务常选择 Rust 而不是 Python？"),
    )
    .with_thinking(ThinkingType::enabled())
    .with_max_tokens(800);

    let response: ChatCompletionResponse = request.send_via(&client).await?;
    let message = response
        .choices()
        .and_then(|choices| choices.first())
        .ok_or("chat response did not contain a choice")?
        .message()
        .ok_or("chat choice omitted its message")?;

    if let Some(reasoning) = message.reasoning_content() {
        println!("reasoning:\n{reasoning}\n");
    }
    println!(
        "answer:\n{}",
        message
            .content_str()
            .ok_or("chat response did not contain an answer")?
    );

    Ok(())
}
