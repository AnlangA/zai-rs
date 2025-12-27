use anyhow::{anyhow, Result};
use reqwest::header::{ACCEPT, AUTHORIZATION};
use serde_json::Value;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".to_string().into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let api_key = std::env::var("ZHIPU_API_KEY").map_err(|_| {
        anyhow!("ZHIPU_API_KEY is not set. Please export your API key.")
    })?;

    let client = reqwest::Client::default();
    let url = "https://open.bigmodel.cn/api/mcp/web_search_prime/mcp";

    tracing::info!("Connecting to MCP server: {}", url);

    // Send tools/list request
    let tools_request = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/list",
        "params": {}
    });

    let tools_response = client
        .post(url)
        .header(ACCEPT, "application/json, text/event-stream")
        .header(AUTHORIZATION, format!("Bearer {}", api_key))
        .json(&tools_request)
        .send()
        .await?;

    let tools_text = tools_response.text().await?;
    tracing::info!("Tools response: {}", tools_text);

    // Parse tools from SSE response
    let json_obj = parse_sse_response(&tools_text)?;

    if let Some(result) = json_obj.get("result").and_then(|r| r.get("tools")) {
        let tools: Vec<Value> = serde_json::from_value(result.clone())?;
        println!("\n=== MCP Server Tools ===\n");
        for tool in &tools {
            let name = tool["name"].as_str().unwrap_or("unknown");
            println!("Tool: {}", name);
            if let Some(desc) = tool.get("description").and_then(|v| v.as_str()) {
                println!("  Description: {}", desc);
            }
            // Check both "inputSchema" (from response) and "input_schema" (common alias)
            if let Some(schema) = tool.get("inputSchema").or_else(|| tool.get("input_schema")) {
                println!("  Input Schema: {}", serde_json::to_string_pretty(schema)?);
            }
            println!();
        }
        println!("Total: {} tools", tools.len());
    } else {
        anyhow::bail!("Failed to get tools from response");
    }

    Ok(())
}

fn parse_sse_response(text: &str) -> Result<Value> {
    for line in text.lines() {
        if let Some(data) = line.strip_prefix("data:") {
            if let Ok(json) = serde_json::from_str::<Value>(data.trim()) {
                return Ok(json);
            }
        }
    }
    anyhow::bail!("No valid data found in SSE response")
}
