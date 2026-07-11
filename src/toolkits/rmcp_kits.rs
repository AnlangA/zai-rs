//! RMCP bridge utilities for zai-rs
//!
//! This module reduces the complexity of integrating RMCP tools with zai-rs
//! by providing:
//! - Conversion from `rmcp::model::Tool` to zai-rs function-call tool defs
//! - Simple helpers to call RMCP tools and normalize results
//! - A small caller struct to encapsulate `ServerSink` usage
//!
//! All APIs are feature-gated behind `rmcp-kits`.
//!
//! Example: convert RMCP tools and wire them into a chat request
//! ```rust,ignore
//! use rmcp::{ServiceExt, model::ClientInfo, transport::SseClientTransport};
//! use zai_rs::{model::{Tools, Function}, toolkits::rmcp_kits};
//! # async fn demo() -> anyhow::Result<()> {
//! let transport = SseClientTransport::start("http://localhost:8000/sse").await?;
//! let client = ClientInfo::default().serve(transport).await?;
//! let server = client.peer().clone();
//! let tools = server.list_all_tools().await?;
//! // Convert RMCP tools to zai-rs function-call tools
//! let tool_defs: Vec<Tools> = rmcp_kits::mcp_tools_to_functions(&tools);
//! # Ok(()) }
//! ```
//!
//! Example: execute a tool call and collect results by tool name
//! ```rust,ignore
//! use rmcp::service::ServerSink;
//! use zai_rs::toolkits::rmcp_kits::{call_mcp_tool, call_mcp_tools_collect};
//! # async fn run(server: &ServerSink) -> anyhow::Result<()> {
//! let (name, value) = call_mcp_tool(server, "increment", Some(serde_json::json!({"n": 2}))).await?;
//! let collected = call_mcp_tools_collect(server, vec![
//!     ("increment".to_string(), Some(serde_json::json!({"n": 1}))),
//!     ("increment".to_string(), Some(serde_json::json!({"n": 3}))),
//! ]).await?;
//! # Ok(()) }
//! ```

use std::collections::HashMap;

use rmcp::{
    model::{CallToolRequestParams, CallToolResult, Tool},
    service::ServerSink,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::{debug, warn};
use validator::Validate;

use crate::{
    client::error::codes,
    model::{Function, Tools},
};

/// Convert a single RMCP tool to a zai-rs function-call definition.
///
/// - Name and description are carried over
/// - Parameters schema is taken from RMCP `input_schema`
#[inline]
pub fn mcp_tool_to_function(t: &Tool) -> Tools {
    let desc = t.description.as_deref().unwrap_or("Remote MCP tool");
    let schema = t.schema_as_json_value();
    Tools::Function {
        function: Function::new(t.name.to_string(), desc.to_string(), schema),
    }
}

/// Convert a list of RMCP tools to zai-rs function-call definitions.
#[inline]
pub fn mcp_tools_to_functions(tools: &[Tool]) -> Vec<Tools> {
    tools.iter().map(mcp_tool_to_function).collect()
}

/// Normalize a CallToolResult to a compact JSON payload suitable for LLM tool
/// results.
///
/// Preference order:
/// 1) `structured_content` if present
/// 2) Fallback: serialize the whole result
#[inline]
pub fn call_tool_result_to_json(res: &CallToolResult) -> Value {
    if let Some(structured) = &res.structured_content {
        return structured.clone();
    }
    serde_json::to_value(res).unwrap_or_else(|_| {
        serde_json::json!({
            "error": {"type": "serialization_error", "message": "failed to serialize tool result"}
        })
    })
}

/// Request payload for calling a single MCP tool.
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct McpCallSpec {
    /// Tool name (non-empty)
    #[validate(length(min = 1))]
    pub name: String,
    /// JSON arguments; must be an object when provided
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<Value>,
}

impl McpCallSpec {
    /// Create a new `McpCallSpec` from a tool name and optional JSON arguments.
    ///
    /// `arguments` should be a JSON object when `Some`; pass `None` for a
    /// parameterless tool.
    pub fn new(name: impl Into<String>, arguments: Option<Value>) -> Self {
        Self {
            name: name.into(),
            arguments,
        }
    }
}

/// Call a single MCP tool and return (tool name, normalized JSON result).
pub async fn call_mcp_tool(
    server: &ServerSink,
    name: impl Into<String>,
    args: Option<Value>,
) -> crate::ZaiResult<(String, Value)> {
    // Validate name and normalize args
    let name: String = name.into();
    if name.trim().is_empty() {
        warn!(
            code = codes::SDK_VALIDATION,
            "MCP tool call rejected: empty tool name"
        );
        return Err(crate::client::error::ZaiError::Unknown {
            code: codes::SDK_VALIDATION,
            message: "tool name cannot be empty".to_string(),
        });
    }
    let arguments = match args {
        Some(Value::Object(map)) => Some(map),
        Some(other) => {
            // Keep interface forgiving: warn by encoding into a wrapper error object
            let val = serde_json::json!({
                "error": {"type": "invalid_arguments", "message": "arguments must be a JSON object", "got": other}
            });
            return Ok((name.clone(), val));
        },
        None => None,
    };

    let mut request = CallToolRequestParams::new(name.clone());
    if let Some(arguments) = arguments {
        request = request.with_arguments(arguments);
    }

    let res = server.call_tool(request).await.map_err(|e| {
        warn!(
            code = codes::SDK_EXTERNAL_TOOL,
            tool = %name,
            error = %e,
            "RMCP call_tool failed"
        );
        crate::client::error::ZaiError::Unknown {
            code: codes::SDK_EXTERNAL_TOOL,
            message: format!("RMCP service error: {e}"),
        }
    })?;
    Ok((name, call_tool_result_to_json(&res)))
}

/// Batch-call multiple tools and collect results by tool name.
/// If multiple calls share the same name, later results overwrite earlier ones.
///
/// **All-or-nothing:** returns `Err` on the first failing tool call, which
/// cancels any still-in-flight calls; sibling results that have not yet been
/// collected are discarded. If you need partial results across failures, drive
/// the calls individually via [`call_mcp_tool`].
pub async fn call_mcp_tools_collect<I>(
    server: &ServerSink,
    calls: I,
) -> crate::ZaiResult<HashMap<String, Value>>
where
    I: IntoIterator<Item = (String, Option<Value>)>,
{
    use futures_util::stream::{FuturesUnordered, StreamExt};
    let mut futs = FuturesUnordered::new();
    for (name, args) in calls {
        futs.push(call_mcp_tool(server, name, args));
    }
    let mut map = HashMap::new();
    while let Some(item) = futs.next().await {
        let (name, value) = item?;
        map.insert(name, value);
    }
    Ok(map)
}

/// A small helper that encapsulates a server handle and provides a concise call
/// API.
#[derive(Clone)]
pub struct McpToolCaller {
    server: ServerSink,
}

impl McpToolCaller {
    /// Create a new tool caller from a server sink.
    pub fn new(server: ServerSink) -> Self {
        Self { server }
    }

    /// Call a tool by name.
    pub async fn call(
        &self,
        name: impl Into<String>,
        args: Option<Value>,
    ) -> crate::ZaiResult<(String, Value)> {
        call_mcp_tool(&self.server, name, args).await
    }

    /// Batch call tools and collect results.
    pub async fn call_collect<I>(&self, calls: I) -> crate::ZaiResult<HashMap<String, Value>>
    where
        I: IntoIterator<Item = (String, Option<Value>)>,
    {
        call_mcp_tools_collect(&self.server, calls).await
    }
}

/// Execute tool calls requested by the first choice in a ChatCompletionResponse
/// and build tool messages ready to append to the chat.
///
/// This encapsulates:
/// - Extracting tool_calls from the assistant message
/// - Parsing function name and JSON arguments safely
/// - Executing the RMCP tool via McpToolCaller
/// - Packaging results as TextMessage::tool_with_id
///
/// Returns an empty Vec when there are no tool calls.
#[cfg(feature = "rmcp-kits")]
pub async fn execute_tool_calls_as_messages(
    caller: &McpToolCaller,
    resp: &crate::model::chat_base_response::ChatCompletionResponse,
) -> crate::ZaiResult<Vec<crate::model::chat_message_types::TextMessage>> {
    use crate::model::{chat_base_response::ToolCallMessage, chat_message_types::TextMessage};

    let mut out: Vec<TextMessage> = Vec::new();
    let calls: Option<&[ToolCallMessage]> = resp
        .choices()
        .and_then(|v| v.first())
        .and_then(|c| c.message().tool_calls());

    let Some(calls) = calls else { return Ok(out) };
    debug!(tool_calls = calls.len(), "Dispatching tool calls");

    for tc in calls {
        // Extract tool call id
        let id = match tc.id() {
            Some(id) => id.to_string(),
            None => {
                warn!(reason = "missing_id", "Skipping tool call");
                continue;
            },
        };

        // Extract function payload
        let func = match tc.function() {
            Some(f) => f,
            None => {
                warn!(reason = "missing_function", "Skipping tool call");
                continue;
            },
        };

        // Name must be present
        let name = match func.name() {
            Some(n) => n.to_string(),
            None => {
                warn!(reason = "missing_function_name", "Skipping tool call");
                continue;
            },
        };

        // Parse JSON arguments if present, and only accept JSON object
        let args_value: Option<serde_json::Value> = match func.arguments() {
            Some(arg_str) => match serde_json::from_str::<serde_json::Value>(arg_str) {
                Ok(serde_json::Value::Object(map)) => Some(serde_json::Value::Object(map)),
                Ok(_) => {
                    warn!(
                        reason = "non_object_arguments",
                        "Function arguments are not an object; passing None"
                    );
                    None
                },
                Err(e) => {
                    warn!(reason = "invalid_arguments_json", error = %e, "Failed to parse function arguments JSON");
                    None
                },
            },
            None => None,
        };

        // Call RMCP server via rmcp-kits
        let (_tool, payload): (String, Value) =
            caller.call(name, args_value).await.map_err(|e| {
                crate::client::error::ZaiError::Unknown {
                    code: codes::SDK_EXTERNAL_TOOL,
                    message: format!("RMCP call_tool failed: {e}"),
                }
            })?;

        // Wrap tool result as a tool message with id
        out.push(TextMessage::tool_with_id(payload.to_string(), id));
    }

    Ok(out)
}

/// Perform a complete MCP tool-call roundtrip:
/// - Send the first chat request
/// - Execute any requested tool calls via MCP
/// - Append tool results as tool messages
/// - Disable tools and add an optional system hint
/// - Send the second request and return the final response
///
/// If no tool calls are requested, returns the first response directly.
#[cfg(feature = "rmcp-kits")]
pub async fn run_mcp_tool_roundtrip<N>(
    caller: &McpToolCaller,
    client: &crate::client::ZaiClient,
    mut chat: crate::model::chat::data::ChatCompletion<
        N,
        crate::model::chat_message_types::TextMessage,
        crate::model::traits::StreamOff,
    >,
    system_hint_after_tools: Option<&str>,
) -> crate::ZaiResult<crate::model::chat_base_response::ChatCompletionResponse>
where
    N: crate::model::traits::ModelName + crate::model::traits::Chat + serde::Serialize,
    (N, crate::model::chat_message_types::TextMessage): crate::model::traits::Bounded,
{
    use crate::model::chat_message_types::TextMessage;

    let first_resp = chat.send_via(client).await?;

    let tool_msgs: Vec<crate::model::chat_message_types::TextMessage> =
        execute_tool_calls_as_messages(caller, &first_resp).await?;

    if tool_msgs.is_empty() {
        return Ok(first_resp);
    }

    for m in tool_msgs {
        chat = chat.add_messages(m);
    }

    // Disable tools for the second round to encourage final answer
    chat.body_mut().tools = None;

    if let Some(hint) = system_hint_after_tools {
        chat = chat.add_messages(TextMessage::system(hint));
    }

    let final_resp = chat.send_via(client).await?;
    Ok(final_resp)
}

/// Extract a concise final text from ChatCompletionResponse when possible.
/// - If content is a string, return it
/// - If content is an array, return the first item of type "text"'s `text`
///   field
/// - Otherwise return None
#[cfg(feature = "rmcp-kits")]
pub fn extract_final_text(
    resp: &crate::model::chat_base_response::ChatCompletionResponse,
) -> Option<String> {
    let msg = resp.choices()?.first()?.message();
    match msg.content() {
        Some(serde_json::Value::String(s)) => Some(s.clone()),
        Some(serde_json::Value::Array(arr)) => arr.iter().find_map(|item| {
            if let serde_json::Value::Object(obj) = item
                && obj.get("type").and_then(|v| v.as_str()) == Some("text")
            {
                return obj
                    .get("text")
                    .and_then(|v| v.as_str())
                    .map(std::string::ToString::to_string);
            }
            None
        }),
        _ => None,
    }
}
