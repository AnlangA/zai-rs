//! Analyze one local or remote image through the Vision MCP backend.

use zai_rs::mcp::{AnalyzeImageRequest, McpClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let image = std::env::args()
        .nth(1)
        .ok_or("usage: mcp_vision <image-path-or-url>")?;
    let request = AnalyzeImageRequest::new(image, "Describe this image comprehensively.");

    let client = McpClient::from_env()?;
    let response = client.analyze_image_with(request).await;
    client.close().await?;
    println!("{}", response?);

    Ok(())
}
