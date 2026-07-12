//! Search the web through the managed MCP backend with typed options.

use zai_rs::mcp::{McpClient, SearchContentSize, SearchRecency, WebSearchRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let query = args
        .next()
        .ok_or("usage: mcp_web_search <query> [domain]")?;
    let mut request = WebSearchRequest::new(query)
        .recency(SearchRecency::OneMonth)
        .content_size(SearchContentSize::High);
    if let Some(domain) = args.next() {
        request = request.domain(domain);
    }

    let client = McpClient::from_env()?;
    let response = client.web_search_with(request).await;
    client.close().await?;
    println!("{}", serde_json::to_string_pretty(&response?)?);

    Ok(())
}
