//! Discover the MCP tools available through the SDK's managed backends.

use zai_rs::mcp::McpClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = McpClient::from_env()?;
    let tools = client.tools().await;
    client.close().await?;

    for tool in tools? {
        println!("{}", tool.name);
    }
    Ok(())
}
