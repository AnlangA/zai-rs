//! GLM-5.3 thinking example using `ZaiClient`.
//!
//! GLM-5.3 always thinks: `thinking.type` accepts only `enabled`, and
//! `reasoning_effort` selects one of three depths — `low` (light),
//! `high` (enhanced), or `max` (deep, the default). Disabling thinking is
//! rejected by request validation before the request leaves the client.

use zai_rs::{
    client::ZaiClient,
    model::{
        GLM5_3, ReasoningEffort, TextMessage, chat::ChatCompletion,
        chat_base_response::ChatCompletionResponse, tools::ThinkingType,
    },
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = ZaiClient::from_env()?;
    let user_text = "Refactor this module and explain each change.";

    // Recommended shape for coding and other complex tasks:
    // thinking enabled + reasoning_effort = max.
    let request = ChatCompletion::new(GLM5_3 {}, TextMessage::user(user_text))
        .with_thinking(ThinkingType::enabled())
        .with_reasoning_effort(ReasoningEffort::Max);

    // ThinkingType::disabled() would fail validation here with
    // `thinking_cannot_be_disabled` — GLM-5.3 dropped that mode. Requests
    // that used to disable thinking should migrate to `enabled()` plus
    // `ReasoningEffort::Low` instead.

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
