//! GLM-4.5 Thinking Mode Example (P05: routes through ZaiClient).

use zai_rs::client::v2::ZaiClient;
use zai_rs::model::{chat_base_response::ChatCompletionResponse, *};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = ZaiClient::from_env()?;

    println!("=== GLM-4.5 Thinking Mode Demo ===\n");

    println!("📝 Example 1: Simple Question (Thinking Disabled)");
    let response = simple_question(&client).await?;
    print_response(&response);

    println!("\n📝 Example 2: Medium Complexity (Thinking Enabled)");
    let response = medium_question(&client).await?;
    print_response(&response);

    println!("\n📝 Example 3: Complex Reasoning (Thinking Enabled)");
    let response = complex_question(&client).await?;
    print_response(&response);

    if let Some(usage) = response.usage {
        println!("\n📊 Token Usage:");
        if let Some(prompt) = usage.prompt_tokens() {
            println!("  Prompt tokens: {prompt}");
        }
        if let Some(completion) = usage.completion_tokens() {
            println!("  Completion tokens: {completion}");
        }
        if let Some(total) = usage.total_tokens() {
            println!("  Total tokens: {total}");
        }
    }

    println!("\n=== Demo Complete ===");
    Ok(())
}

fn print_response(response: &ChatCompletionResponse) {
    if let Some(choices) = response.choices.as_ref()
        && let Some(choice) = choices.first()
    {
        if let Some(reasoning) = choice.message().reasoning_content() {
            println!("🤔 Thinking Process:\n{reasoning}\n---\n");
        }
        if let Some(content) = choice.message().content() {
            println!("💡 Answer: {content}\n");
        }
    }
}

async fn simple_question(
    client: &ZaiClient,
) -> Result<ChatCompletionResponse, Box<dyn std::error::Error>> {
    let request = ChatCompletion::new(
        GLM4_5_flash {},
        TextMessage::user("What is the capital of China? Please answer in one sentence."),
    )
    .with_thinking(ThinkingType::disabled())
    .with_max_tokens(100);
    request.send_via(client).await.map_err(Into::into)
}

async fn medium_question(
    client: &ZaiClient,
) -> Result<ChatCompletionResponse, Box<dyn std::error::Error>> {
    let request = ChatCompletion::new(
        GLM4_5 {},
        TextMessage::user("Why might a business choose to use Rust over Python for a new project?"),
    )
    .with_thinking(ThinkingType::enabled())
    .with_temperature(0.7)
    .with_max_tokens(500);
    request.send_via(client).await.map_err(Into::into)
}

async fn complex_question(
    client: &ZaiClient,
) -> Result<ChatCompletionResponse, Box<dyn std::error::Error>> {
    let request = ChatCompletion::new(
        GLM4_5 {},
        TextMessage::user("Explain how mixture-of-experts (MoE) models work in detail."),
    )
    .with_thinking(ThinkingType::enabled())
    .with_temperature(0.5)
    .with_max_tokens(2000);
    request.send_via(client).await.map_err(Into::into)
}
