//! Exercise every Web Reader MCP request option.
//!
//! Run with:
//! cargo run --example mcp_web_reader --features mcp -- https://docs.rs/rmcp/2.2.0/rmcp/

use std::env;

use zai_rs::mcp::{McpClient, WebReaderFormat, WebReaderRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let url = env::args()
        .nth(1)
        .unwrap_or_else(|| "https://docs.rs/rmcp/2.2.0/rmcp/".to_owned());

    let client = McpClient::from_env()?;
    let request = WebReaderRequest::new(url)
        .format(WebReaderFormat::Markdown)
        .retain_images(true)
        .links_summary(true)
        .images_summary(true)
        .keep_image_data_urls(false)
        .github_flavored_markdown(true)
        .cache(false)
        .timeout_seconds(60);

    let response = client.read_web_page_with(request).await?;
    println!("{}", serde_json::to_string_pretty(&response)?);

    client.close().await?;
    Ok(())
}
