//! List every official MCP tool through the unified high-level API.

use anyhow::Result;
use zai_rs::mcp::McpClient;

#[tokio::main]
async fn main() -> Result<()> {
    let client = McpClient::from_env()?;
    let tools = client.tools().await?;

    println!("=== Official MCP tools ===\n");
    for tool in &tools {
        println!("Tool: {}", tool.name);
        if let Some(description) = &tool.description {
            println!("  Description: {description}");
        }
        println!(
            "  Input Schema: {}",
            serde_json::to_string_pretty(&tool.input_schema)?
        );
        println!();
    }
    println!("Total: {} tools", tools.len());

    client.close().await?;
    Ok(())
}
