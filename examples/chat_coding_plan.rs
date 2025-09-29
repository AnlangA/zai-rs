//! # Chat Coding Plan Example
//!
//! This example demonstrates how to use the ZAI-RS SDK with the coding plan API endpoint
//! for specialized coding assistance with the Zhipu AI API.
//!
//! ## Features Demonstrated
//!
//! - Model selection (GLM-4.5-Flash)
//! - Text message creation
//! - Coding plan endpoint configuration
//! - Request parameter configuration
//! - Response handling and parsing
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
//! cargo run --example chat_coding_plan
//! ```

use zai_rs::model::chat_base_response::ChatCompletionResponse;
use zai_rs::model::*;

use tokio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging for debugging
    env_logger::init();

    // Select the AI model - GLM-4.5-Flash for fast, efficient responses
    let model = GLM4_5_flash {};

    // Get API key from environment variable
    let key = std::env::var("ZHIPU_API_KEY").expect("ZHIPU_API_KEY must be set");

    // User input for coding assistance (Chinese: "Help me write a Rust function to calculate factorial")
    let user_text = "帮我写一个计算阶乘的 Rust 函数。只返回函数。其他内容不要返回";

    // Build the chat completion request with coding plan endpoint
    let client = ChatCompletion::new(model, TextMessage::user(user_text), key).with_coding_plan();
    // Send the request and await response (non-stream)
    let body: ChatCompletionResponse = client.send().await?;
    println!("{:#?}", body);

    Ok(())
}
