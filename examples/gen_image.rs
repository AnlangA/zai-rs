//! # Image Generation Example
//!
//! This example demonstrates how to use the ZAI-RS SDK for AI-powered image
//! generation using the CogView4 model from Zhipu AI.
//!
//! ## Features Demonstrated
//!
//! - Image model selection (CogView4)
//! - Text prompt creation for image generation
//! - Image size configuration
//! - Request building and submission
//! - Response handling with generated image data
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
//! cargo run --example gen_image
//! ```

use zai_rs::model::gen_image::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging for debugging
    env_logger::init();

    // Prepare API key and model
    let key = std::env::var("ZHIPU_API_KEY").expect("Please set ZHIPU_API_KEY env var");
    let model = CogView4 {};

    // Example prompt and size (equivalent to the curl example)
    // Chinese: "A cute little kitten sitting on a sunny windowsill with blue sky
    // and white clouds in the background"
    let prompt = "一只可爱的小猫咪，坐在阳光明媚的窗台上，背景是蓝天白云.";
    let size = ImageSize::Size1024x1024;

    // Build request and send
    let client = ImageGenRequest::new(model, key)
        .with_prompt(prompt)
        .with_size(size);

    // Send the request and await the generated image
    let body: ImageResponse = client.send().await?;

    // Display the response containing image information
    println!("{:#?}", body);

    Ok(())
}
