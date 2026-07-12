//! Read one web page through the managed MCP backend as Markdown.

use zai_rs::mcp::{McpClient, WebReaderFormat, WebReaderRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let url = std::env::args()
        .nth(1)
        .ok_or("usage: mcp_web_reader <url>")?;
    let request = WebReaderRequest::new(url)
        .format(WebReaderFormat::Markdown)
        .github_flavored_markdown(true)
        .timeout_seconds(60);

    let client = McpClient::from_env()?;
    let response = client.read_web_page_with(request).await;
    client.close().await?;
    println!("{}", serde_json::to_string_pretty(&response?)?);

    Ok(())
}
