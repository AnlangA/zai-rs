//! Search documentation for a public GitHub repository through ZRead MCP.

use zai_rs::mcp::{McpClient, RepositoryLanguage, SearchDocRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let repository = args
        .next()
        .ok_or("usage: mcp_zread <owner/repository> <query>")?;
    let query = args
        .next()
        .ok_or("usage: mcp_zread <owner/repository> <query>")?;
    let request = SearchDocRequest::new(repository, query).language(RepositoryLanguage::En);

    let client = McpClient::from_env()?;
    let response = client.search_repo_with(request).await;
    client.close().await?;
    println!("{}", response?);

    Ok(())
}
