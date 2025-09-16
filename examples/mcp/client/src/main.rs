use anyhow::{anyhow, Context, Result};
use rmcp::{
    ServiceExt,
    model::{ClientCapabilities, ClientInfo, Implementation},
    service::ServerSink,
    transport::SseClientTransport,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

// ZAI (zai-rs) imports
use zai_rs::model::{chat_base_response::ChatCompletionResponse, *};
// rmcp-kits bridge imports
use zai_rs::toolkits::rmcp_kits::{
    mcp_tools_to_functions, McpToolCaller, run_mcp_tool_roundtrip, extract_final_text,
};

// No toolkits: we'll directly map RMCP tools to ZAI function definitions,
// and manually execute tool calls by forwarding to the RMCP server.

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("info,{}=debug", env!("CARGO_CRATE_NAME")).into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // 1) Connect to MCP server via SSE
    let transport = SseClientTransport::start("http://localhost:8000/sse").await?;
    let client_info = ClientInfo {
        protocol_version: Default::default(),
        capabilities: ClientCapabilities::default(),
        client_info: Implementation {
            name: "test sse client".to_string(),
            title: None,
            version: "0.0.1".to_string(),
            website_url: None,
            icons: None,
        },
    };
    let client = client_info.serve(transport).await.inspect_err(|e| {
        tracing::error!("client error: {:?}", e);
    })?;

    // Initialize
    tracing::info!("Connected to server: {:#?}", client.peer_info());

    // Grab a clonable server handle for tool execution
    let server: ServerSink = client.peer().clone();
    let caller = McpToolCaller::new(server.clone());

    // 2) Retrieve available tools from the server
    let tools = server
        .list_all_tools()
        .await
        .context("failed to list tools from server")?;
    tracing::info!("Available tools: {:#?}", tools.iter().map(|t| &t.name).collect::<Vec<_>>());

    // 3) Convert RMCP tools into ZAI function-call tool definitions (via rmcp-kits)
    let tool_defs: Vec<Tools> = mcp_tools_to_functions(&tools);

    // 4) Ask the AI to perform an increment operation using those tools
    let key = std::env::var("ZHIPU_API_KEY").map_err(|_| anyhow!(
        "ZHIPU_API_KEY is not set. Please export your API key to use the Zhipu AI service."
    ))?;

    let user_text = "Please increment the counter by 2.";
    let chat = ChatCompletion::new(GLM4_5_flash {}, TextMessage::user(user_text), key)
        .with_thinking(ThinkingType::Disabled)
        .add_tools(tool_defs)
        .with_max_tokens(256);

    // 5-7) Full roundtrip (first request -> MCP tools -> second request)
    let final_resp: ChatCompletionResponse = run_mcp_tool_roundtrip(
        &caller,
        chat,
        Some("Now provide the final result to the user based on the tool outputs."),
    )
    .await
    .context("MCP tool-call roundtrip failed")?;
    tracing::info!("AI final response: {:#?}", final_resp);

    // Print concise final text if available
    if let Some(answer) = extract_final_text(&final_resp) {
        println!("Final answer: {}", answer);
    } else {
        println!("Final answer (raw): {:#?}", final_resp);
    }

    // Clean shutdown
    client.cancel().await?;
    Ok(())
}
