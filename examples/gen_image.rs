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

use zai_rs::client::v2::ZaiClient;
use zai_rs::model::gen_image::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Shared client reads ZHIPU_API_KEY from the environment.
    let client = ZaiClient::from_env()?;
    let model = CogView4 {};

    // Example prompt and size (equivalent to the curl example)
    // Chinese: "A cute little kitten sitting on a sunny windowsill with blue sky
    // and white clouds in the background"
    let prompt = "一只可爱的小猫咪，坐在阳光明媚的窗台上，背景是蓝天白云.";
    let size = ImageSize::Size1024x1024;

    // Build request (P05: no key/config on the request; credentials live on the client).
    let request = ImageGenRequest::new(model)
        .with_prompt(prompt)
        .with_size(size);

    // Send the request and await the generated image
    let body: ImageResponse = request.send_via(&client).await?;

    // Display the response containing image information
    println!("{body:#?}");

    Ok(())
}
