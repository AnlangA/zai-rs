//! GLM-5.2 reasoning-effort example using `ZaiClient`.

use zai_rs::{
    client::ZaiClient,
    model::{
        GLM5_2, ReasoningEffort, TextMessage, chat::ChatCompletion,
        chat_base_response::ChatCompletionResponse,
    },
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = ZaiClient::from_env()?;
    let user_text = "Prove that the square root of 2 is irrational.";

    let request = ChatCompletion::new(GLM5_2 {}, TextMessage::user(user_text))
        .with_reasoning_effort(ReasoningEffort::High);
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
