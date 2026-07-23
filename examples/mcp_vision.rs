//! Analyze one local or remote image through the Vision MCP backend.
//!
//! Defaults to `data/短发女.jpeg`; pass a path or URL to use another image.

use zai_rs::{
    mcp::{AnalyzeImageRequest, McpClient},
    model::GLM5V_turbo,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let image = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "data/短发女.jpeg".to_owned());
    let request = AnalyzeImageRequest::new(image, "Describe this image comprehensively.");

    let client = McpClient::from_env()?.with_vision_model(GLM5V_turbo {});
    let response = client.analyze_image_with(request).await;
    client.close().await?;
    println!("{}", response?);

    Ok(())
}
