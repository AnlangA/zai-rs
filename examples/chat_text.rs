//! # Basic Chat Text Example
//!
//! This example demonstrates how to use the ZAI-RS SDK for basic text-based
//! chat completion with the Zhipu AI API.
//!
//! ## Features Demonstrated
//!
//! - Model selection (GLM-4.5-Flash)
//! - Text message creation
//! - Request parameter configuration
//! - Response handling and parsing
//! - Thinking capability control
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
//! cargo run --example chat_text
//! ```

use zai_rs::model::{chat_base_response::ChatCompletionResponse, *};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging for debugging
    env_logger::init();

    // Select the AI model - GLM-4.5-Flash for fast, efficient responses
    let model = GLM4_5_flash {};

    // Get API key from environment variable
    let key = std::env::var("ZHIPU_API_KEY").expect("ZHIPU_API_KEY must be set");

    // User input text (Chinese: "Hello")
    let user_text = "你好";

    // Build the chat completion request with custom parameters
    let client = ChatCompletion::new(model, TextMessage::user(user_text), key)
        .with_temperature(0.7) // Control randomness (0.0-1.0)
        .with_top_p(0.9) // Control diversity (0.0-1.0)
        .with_thinking(ThinkingType::Disabled); // Disable thinking for faster response

    // Send the request and await response (non-stream)
    let body: ChatCompletionResponse = client.send().await?;
    println!("{:#?}", body);

    Ok(())
}
