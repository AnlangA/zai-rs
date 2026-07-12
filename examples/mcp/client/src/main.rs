//! Low-level `rmcp-kits` interoperability example for an arbitrary custom MCP
//! server. Applications using the official Z.AI MCP capabilities should use
//! `zai_rs::mcp::McpClient`, which hides server and transport selection.

use anyhow::{Context, Result};
use rmcp::{
    ServiceExt, model::ClientInfo, service::ServerSink, transport::StreamableHttpClientTransport,
};
// ZAI (zai-rs) imports
use zai_rs::client::ZaiClient;
use zai_rs::model::{chat_base_response::ChatCompletionResponse, *};
// rmcp-kits bridge imports
use zai_rs::toolkits::rmcp_kits::{
    McpToolCaller, extract_final_text, mcp_tools_to_functions, run_mcp_tool_roundtrip,
};

// No ToolExecutor is needed: rmcp-kits maps definitions and forwards calls
// directly to the MCP server.

#[tokio::main]
async fn main() -> Result<()> {
    // 1) Connect to MCP server via streamable HTTP
    let transport = StreamableHttpClientTransport::from_uri("http://localhost:8000/mcp");
    let client_info = ClientInfo::default();
    let client = client_info
        .serve(transport)
        .await
        .context("failed to connect to the MCP server")?;

    // Initialize
    println!("Connected to server: {:#?}", client.peer_info());

    // Grab a clonable server handle for tool execution
    let server: ServerSink = client.peer().clone();
    let caller = McpToolCaller::new(server.clone());

    // 2) Retrieve available tools from the server
    let tools = server
        .list_all_tools()
        .await
        .context("failed to list tools from server")?;
    println!(
        "Available tools: {:#?}",
        tools.iter().map(|t| &t.name).collect::<Vec<_>>()
    );

    // 3) Convert RMCP tools into ZAI function-call tool definitions (via rmcp-kits)
    let tool_defs: Vec<Tools> = mcp_tools_to_functions(&tools);

    // 4) Ask the AI to perform an increment operation using those tools.
    //    The shared ZaiClient supplies credentials and transport to chat requests.
    let zai_client = ZaiClient::from_env()?;

    let user_text = "Please increment the counter by 2.";
    let chat = ChatCompletion::new(GLM4_5_flash {}, TextMessage::user(user_text))
        .with_thinking(ThinkingType::disabled())
        .add_tools(tool_defs)
        .with_max_tokens(256);

    // 5-7) Full roundtrip (first request -> MCP tools -> second request)
    let final_resp: ChatCompletionResponse = run_mcp_tool_roundtrip(
        &caller,
        &zai_client,
        chat,
        Some("Now provide the final result to the user based on the tool outputs."),
    )
    .await
    .context("MCP tool-call roundtrip failed")?;
    println!("AI final response: {final_resp:#?}");

    // Print concise final text if available
    if let Some(answer) = extract_final_text(&final_resp) {
        println!("Final answer: {answer}");
    } else {
        println!("Final answer (raw): {final_resp:#?}");
    }

    // Clean shutdown
    client.cancel().await?;
    Ok(())
}
