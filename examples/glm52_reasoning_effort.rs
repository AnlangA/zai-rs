//! # GLM-5.2 Reasoning Effort Example
//!
//! This example demonstrates the GLM-5.2 flagship model and its new
//! `reasoning_effort` parameter, which controls the depth of reasoning when
//! thinking mode is enabled.
//!
//! ## Features Demonstrated
//!
//! - Model selection (GLM-5.2)
//! - Thinking mode (`ThinkingType::enabled()`)
//! - Reasoning depth control (`ReasoningEffort`)
//!
//! ## Prerequisites
//!
//! Set the `ZHIPU_API_KEY` environment variable with your API key:
//! ```bash
//! export ZHIPU_API_KEY="your-api-key-here"
//! ```
//!
//! ## Running the Example
//!
//! ```bash
//! cargo run --example glm52_reasoning_effort
//! ```

use zai_rs::model::{chat_base_response::ChatCompletionResponse, *};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let key = std::env::var("ZHIPU_API_KEY").expect("Please set ZHIPU_API_KEY env var");

    println!("=== GLM-5.2 Reasoning Effort Demo ===\n");

    // GLM-5.2 is the flagship model with thinking + reasoning_effort support.
    let model = GLM5_2 {};

    let user_text = "Explain how mixture-of-experts (MoE) models work, and why \
                     they might be more efficient than dense models.";

    println!("📝 Question: {}\n", user_text);

    // Enable thinking and request the maximum reasoning depth. Higher effort
    // yields deeper reasoning at the cost of latency and tokens — recommended
    // for coding and architecture-level tasks.
    let client = ChatCompletion::new(model, TextMessage::user(user_text), key)
        .with_thinking(ThinkingType::enabled())
        .with_reasoning_effort(ReasoningEffort::Max);

    let response: ChatCompletionResponse = client.send().await?;

    if let Some(choices) = response.choices.as_ref()
        && let Some(choice) = choices.first()
    {
        if let Some(reasoning) = choice.message().reasoning_content() {
            println!("🤔 Thinking Process:\n{}\n", reasoning);
            println!("---\n");
        }
        if let Some(content) = choice.message().content() {
            println!("💡 Answer: {}\n", content);
        }
    }

    // Show usage statistics
    if let Some(usage) = response.usage {
        println!("📊 Token Usage:");
        if let Some(prompt) = usage.prompt_tokens() {
            println!("  Prompt tokens: {}", prompt);
        }
        if let Some(completion) = usage.completion_tokens() {
            println!("  Completion tokens: {}", completion);
        }
        if let Some(total) = usage.total_tokens() {
            println!("  Total tokens: {}", total);
        }
    }

    println!("\n=== Demo Complete ===");

    Ok(())
}
