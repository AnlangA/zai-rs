//! GLM-4.5 Thinking Mode Example
//!
//! This example demonstrates the deep thinking mode capabilities of GLM-4.5,
//! which shows step-by-step reasoning for complex problems.

use zai_rs::model::{chat_base_response::ChatCompletionResponse, *};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let key = std::env::var("ZHIPU_API_KEY").expect("Please set ZHIPU_API_KEY env var");

    println!("=== GLM-4.5 Thinking Mode Demo ===\n");

    // Example 1: Simple question (no thinking needed)
    println!("📝 Example 1: Simple Question (Thinking Disabled)");
    println!("Question: What is the capital of China?\n");

    let response = simple_question(&key).await?;

    if let Some(choices) = response.choices.as_ref() {
        if let Some(choice) = choices.first() {
            if let Some(reasoning) = choice.message().reasoning_content() {
                println!("🤔 Thinking Process:\n{}", reasoning);
                println!("\n---\n");
            }
            if let Some(content) = choice.message().content() {
                println!("💡 Answer: {}\n", content);
            }
        }
    }

    // Example 2: Medium complexity (some thinking helpful)
    println!("\n📝 Example 2: Medium Complexity (Thinking Enabled)");
    println!("Question: Why might a business choose to use Rust over Python for a new project?\n");

    let response = medium_question(&key).await?;

    if let Some(choices) = response.choices.as_ref() {
        if let Some(choice) = choices.first() {
            if let Some(reasoning) = choice.message().reasoning_content() {
                println!("🤔 Thinking Process:\n{}", reasoning);
                println!("\n---\n");
            }
            if let Some(content) = choice.message().content() {
                println!("💡 Answer: {}\n", content);
            }
        }
    }

    // Example 3: Complex reasoning (maximum thinking needed)
    println!("\n📝 Example 3: Complex Reasoning (Thinking Enabled)");
    println!(
        "Question: Explain how mixture-of-experts (MoE) models work, and why they might be more efficient than dense models.\n"
    );

    let response = complex_question(&key).await?;

    if let Some(choices) = response.choices.as_ref() {
        if let Some(choice) = choices.first() {
            if let Some(reasoning) = choice.message().reasoning_content() {
                println!("🤔 Thinking Process:\n{}", reasoning);
                println!("\n---\n");
            }
            if let Some(content) = choice.message().content() {
                println!("💡 Answer: {}\n", content);
            }
        }
    }

    // Show usage statistics
    if let Some(usage) = response.usage {
        println!("\n📊 Token Usage:");
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

/// Simple question - thinking disabled for faster response
async fn simple_question(key: &str) -> Result<ChatCompletionResponse, Box<dyn std::error::Error>> {
    let model = GLM4_5_flash {};
    let user_text = "What is the capital of China? Please answer in one sentence.";

    let client = ChatCompletion::new(model, TextMessage::user(user_text), key.to_string())
        .with_thinking(ThinkingType::Disabled)
        .with_max_tokens(100);

    client.send().await.map_err(Into::into)
}

/// Medium complexity question - thinking enabled for better reasoning
async fn medium_question(key: &str) -> Result<ChatCompletionResponse, Box<dyn std::error::Error>> {
    let model = GLM4_5 {};
    let user_text = "Why might a business choose to use Rust over Python for a new project? Consider performance, safety, and ecosystem factors.";

    let client = ChatCompletion::new(model, TextMessage::user(user_text), key.to_string())
        .with_thinking(ThinkingType::Enabled)
        .with_temperature(0.7)
        .with_max_tokens(500);

    client.send().await.map_err(Into::into)
}

/// Complex reasoning question - maximum thinking for deep analysis
async fn complex_question(key: &str) -> Result<ChatCompletionResponse, Box<dyn std::error::Error>> {
    let model = GLM4_5 {};
    let user_text = "Explain how mixture-of-experts (MoE) models work in detail. Include:
1. What is the basic architecture?
2. How do routers decide which experts to use?
3. Why are MoE models more parameter-efficient?
4. What are the trade-offs compared to dense models?

Please provide a comprehensive explanation with examples.";

    let client = ChatCompletion::new(model, TextMessage::user(user_text), key.to_string())
        .with_thinking(ThinkingType::Enabled)
        .with_temperature(0.5)
        .with_max_tokens(2000);

    client.send().await.map_err(Into::into)
}
