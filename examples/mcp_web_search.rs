//! Exercise the complete Web Search MCP request surface.
//!
//! Run with:
//! cargo run --example mcp_web_search --features mcp -- "Rust rmcp" docs.rs

use std::env;

use zai_rs::mcp::{McpClient, SearchContentSize, SearchLocation, SearchRecency, WebSearchRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args().skip(1);
    let query = args.next().unwrap_or_else(|| "Rust rmcp 2.2.0".to_owned());
    let domain = args.next().unwrap_or_else(|| "docs.rs".to_owned());

    let client = McpClient::from_env()?;
    let request = WebSearchRequest::new(query)
        .domain(domain)
        .recency(SearchRecency::OneMonth)
        .content_size(SearchContentSize::High)
        .location(SearchLocation::International);

    let response = client.web_search_with(request).await?;
    println!("{}", serde_json::to_string_pretty(&response)?);

    client.close().await?;
    Ok(())
}
