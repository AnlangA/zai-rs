//! GLM-5.2 Reasoning Effort Example (P05: routes through ZaiClient).
use zai_rs::client::v2::ZaiClient;
use zai_rs::model::{chat_base_response::ChatCompletionResponse, *};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = ZaiClient::from_env()?;
    let user_text = "Prove that the square root of 2 is irrational.";

    let request = ChatCompletion::new(GLM5_2 {}, TextMessage::user(user_text))
        .with_reasoning_effort(ReasoningEffort::High);
    let response: ChatCompletionResponse = request.send_via(&client).await?;
    println!("{response:#?}");
    Ok(())
}
