use anyhow::{anyhow, Context, Result};
use rmcp::{
    ServiceExt,
    model::{CallToolRequestParam, CallToolResult, ClientCapabilities, ClientInfo, Implementation},
    service::ServerSink,
    transport::SseClientTransport,
};
use serde_json::Value;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

// ZAI (zai-rs) imports
use zai_rs::model::{chat_base_response::ChatCompletionResponse, *};

/// Convert an RMCP CallToolResult to a compact JSON payload for LLM tool result consumption.
fn call_tool_result_to_json(res: &CallToolResult) -> Value {
    if let Some(structured) = &res.structured_content {
        return structured.clone();
    }
    // Fallback: serialize the whole result. Most tools at least include text content.
    serde_json::to_value(res).unwrap_or_else(|_| serde_json::json!({
        "error": {"type": "serialization_error", "message": "failed to serialize tool result"}
    }))
}

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

    // 2) Retrieve available tools from the server
    let tools = server
        .list_all_tools()
        .await
        .context("failed to list tools from server")?;
    tracing::info!("Available tools: {:#?}", tools.iter().map(|t| &t.name).collect::<Vec<_>>());

    // 3) Convert RMCP tools into ZAI function-call tool definitions (no toolkits)
    let tool_defs: Vec<Tools> = tools
        .iter()
        .map(|t| {
            let desc = t.description.as_deref().unwrap_or("Remote MCP tool");
            let schema = t.schema_as_json_value();
            Tools::Function { function: Function::new(t.name.to_string(), desc.to_string(), schema) }
        })
        .collect();

    // 4) Ask the AI to perform an increment operation using those tools
    let key = std::env::var("ZHIPU_API_KEY").map_err(|_| anyhow!(
        "ZHIPU_API_KEY is not set. Please export your API key to use the Zhipu AI service."
    ))?;

    let user_text = "Please increment the counter by 2.";
    let mut chat = ChatCompletion::new(GLM4_5_flash {}, TextMessage::user(user_text), key)
        .with_thinking(ThinkingType::Disabled)
        .add_tools(tool_defs)
        .with_max_tokens(256);

    // 5) First LLM round: model selects tool(s)
    let first_resp: ChatCompletionResponse = chat.send().await.context("LLM request failed")?;
    tracing::info!("AI first response: {:#?}", first_resp);

    // 6) If AI requested tool calls, execute them via RMCP and feed results back
    if let Some(calls) = first_resp
        .choices()
        .and_then(|v| v.get(0))
        .and_then(|c| c.message().tool_calls())
    {
        tracing::info!("AI requested tool calls: {}", calls.len());
        for tc in calls {
            // Extract tool call info
            let id = match tc.id() { Some(id) => id.to_string(), None => {
                tracing::warn!("Tool call without id, skipping");
                continue;
            }};
            let func = match tc.function() { Some(f) => f, None => {
                tracing::warn!("Tool call missing function payload, skipping");
                continue;
            }};
            let name = match func.name() { Some(n) => n.to_string(), None => {
                tracing::warn!("Tool call missing function name, skipping");
                continue;
            }};
            // Parse arguments as JSON object if possible
            let arguments: Option<serde_json::Map<String, Value>> = match func.arguments() {
                Some(arg_str) => match serde_json::from_str::<Value>(arg_str) {
                    Ok(Value::Object(map)) => Some(map),
                    Ok(_) => {
                        tracing::warn!("Function arguments are not an object; passing None");
                        None
                    }
                    Err(e) => {
                        tracing::warn!("Failed to parse function arguments JSON: {}", e);
                        None
                    }
                },
                None => None,
            };
            // Call RMCP server
            let res = server
                .call_tool(CallToolRequestParam { name: name.into(), arguments })
                .await
                .context("RMCP call_tool failed")?;
            println!("tool result: {:#?}", res);
            let payload = call_tool_result_to_json(&res);
            chat = chat.add_messages(TextMessage::tool_with_id(payload.to_string(), id));
        }
        // Disable tools for the second round to encourage final answer
        chat.body_mut().tools = None;
        chat = chat.add_messages(TextMessage::system(
            "Now provide the final result to the user based on the tool outputs.",
        ));

        // 7) Second LLM round: final answer
        let final_resp: ChatCompletionResponse = chat.send().await.context("LLM follow-up failed")?;
        tracing::info!("AI final response: {:#?}", final_resp);

        // Print a concise final text if available
        if let Some(msg) = final_resp.choices().and_then(|v| v.get(0)).map(|c| c.message()) {
            let text_opt = match msg.content() {
                Some(serde_json::Value::String(s)) => Some(s.clone()),
                Some(serde_json::Value::Array(arr)) => arr.iter().find_map(|item| {
                    if let serde_json::Value::Object(obj) = item {
                        if obj.get("type").and_then(|v| v.as_str()) == Some("text") {
                            return obj.get("text").and_then(|v| v.as_str()).map(|s| s.to_string());
                        }
                    }
                    None
                }),
                Some(other) => Some(other.to_string()),
                None => None,
            };
            if let Some(answer) = text_opt { println!("Final answer: {}", answer); }
            else { println!("Final answer (raw): {:#?}", final_resp); }
        } else {
            println!("Final answer (raw): {:#?}", final_resp);
        }
    } else {
        tracing::warn!("Model did not request any tool calls. Nothing to execute.");
    }

    // Clean shutdown
    client.cancel().await?;
    Ok(())
}
