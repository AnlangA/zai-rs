//! Use MCP capabilities without selecting or configuring MCP servers.
//!
//! ```console
//! cargo run --example mcp --features mcp -- search "Rust rmcp"
//! cargo run --example mcp --features mcp -- read https://docs.rs/rmcp
//! cargo run --example mcp --features mcp -- tools
//! ```

use std::env;

use zai_rs::mcp::McpClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args().skip(1);
    let operation = args.next().unwrap_or_else(|| "tools".to_owned());
    let client = McpClient::from_env()?;

    let result = match operation.as_str() {
        "search" => serde_json::to_value(
            client
                .web_search(args.next().ok_or("missing search query")?)
                .await?,
        )?,
        "read" => serde_json::to_value(
            client
                .read_web_page(args.next().ok_or("missing URL")?)
                .await?,
        )?,
        "repo-search" => {
            let repository = args.next().ok_or("missing owner/repo")?;
            let query = args.next().ok_or("missing repository query")?;
            serde_json::json!({"text": client.search_repo(repository, query).await?.into_text()})
        },
        "image" => {
            let image = args.next().ok_or("missing image path or URL")?;
            let prompt = args
                .next()
                .unwrap_or_else(|| "Describe this image".to_owned());
            serde_json::json!({"text": client.analyze_image(image, prompt).await?.into_text()})
        },
        "tools" => {
            let tools = client.tools().await?;
            serde_json::json!({
                "tools": tools.iter().map(|tool| tool.name.as_ref()).collect::<Vec<_>>()
            })
        },
        _ => return Err(format!("unknown operation: {operation}").into()),
    };

    println!("{}", serde_json::to_string_pretty(&result)?);
    client.close().await?;
    Ok(())
}
