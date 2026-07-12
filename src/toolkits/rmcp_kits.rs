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
//! ```rust,no_run
//! use rmcp::{
//!     ServiceExt,
//!     model::ClientInfo,
//!     transport::StreamableHttpClientTransport,
//! };
//! use zai_rs::{model::{Tools, Function}, toolkits::rmcp_kits};
//! # async fn demo() -> Result<(), Box<dyn std::error::Error>> {
//! let transport = StreamableHttpClientTransport::from_uri("http://localhost:8000/mcp");
//! let client = ClientInfo::default().serve(transport).await?;
//! let server = client.peer().clone();
//! let tools = server.list_all_tools().await?;
//! // Convert RMCP tools to zai-rs function-call tools
//! let tool_defs: Vec<Tools> = rmcp_kits::mcp_tools_to_functions(&tools);
//! # Ok(()) }
//! ```
//!
//! Example: execute a tool call and collect results by tool name
//! ```rust,no_run
//! use rmcp::service::ServerSink;
//! use zai_rs::toolkits::rmcp_kits::{call_mcp_tool, call_mcp_tools_collect};
//! # async fn run(server: &ServerSink) -> Result<(), Box<dyn std::error::Error>> {
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

/// Convert a `CallToolResult` to JSON suitable for an LLM tool message.
///
/// Preference order:
/// 1) `structured_content` if present
/// 2) Fallback: serialize the whole result
///
/// MCP error results retain their full envelope so the `isError` marker is not
/// lost when structured content is present.
#[inline]
pub fn call_tool_result_to_json(res: &CallToolResult) -> Value {
    if res.is_error == Some(true) {
        return serde_json::to_value(res).unwrap_or_else(|_| serialization_error_value());
    }
    if let Some(structured) = &res.structured_content {
        return structured.clone();
    }
    serde_json::to_value(res).unwrap_or_else(|_| serialization_error_value())
}

fn serialization_error_value() -> Value {
    serde_json::json!({
        "error": {"type": "serialization_error", "message": "failed to serialize tool result"}
    })
}

fn text_tool_message(
    content: String,
    id: Option<&str>,
) -> crate::model::chat_message_types::TextMessage {
    match id {
        Some(id) => crate::model::chat_message_types::TextMessage::tool_with_id(content, id),
        None => crate::model::chat_message_types::TextMessage::tool(content),
    }
}

/// Request payload for calling a single MCP tool.
#[derive(Clone, Serialize, Deserialize)]
pub struct McpCallSpec {
    /// Tool name matching `[A-Za-z0-9_-]{1,64}`.
    pub name: String,
    /// JSON arguments; must be an object when provided
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<Value>,
}

impl std::fmt::Debug for McpCallSpec {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("McpCallSpec")
            .field("name", &self.name)
            .field("arguments", &self.arguments.as_ref().map(|_| "<redacted>"))
            .finish()
    }
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

    /// Validate the tool name and require arguments to be a JSON object when
    /// present.
    pub fn validate(&self) -> crate::ZaiResult<()> {
        crate::toolkits::core::validate_tool_name(&self.name)
            .map_err(|error| validation_error(&error.to_string()))?;
        if self
            .arguments
            .as_ref()
            .is_some_and(|arguments| !arguments.is_object())
        {
            return Err(validation_error("arguments must be a JSON object"));
        }
        Ok(())
    }
}

fn validation_error(message: &str) -> crate::client::error::ZaiError {
    crate::client::error::ZaiError::ApiError {
        code: codes::SDK_VALIDATION,
        message: message.to_string(),
    }
}

/// Call a single MCP tool and return `(tool name, JSON result)`.
///
/// Transport/protocol and local validation failures are returned as [`Err`].
/// MCP in-band tool errors remain encoded in the returned JSON.
pub async fn call_mcp_tool(
    server: &ServerSink,
    name: impl Into<String>,
    args: Option<Value>,
) -> crate::ZaiResult<(String, Value)> {
    let spec = McpCallSpec::new(name, args);
    spec.validate()?;
    let McpCallSpec { name, arguments } = spec;
    let arguments = match arguments {
        Some(Value::Object(arguments)) => Some(arguments),
        None => None,
        Some(_) => return Err(validation_error("arguments must be a JSON object")),
    };

    let mut request = CallToolRequestParams::new(name.clone());
    if let Some(arguments) = arguments {
        request = request.with_arguments(arguments);
    }

    let res = server.call_tool(request).await.map_err(|_| {
        warn!(code = codes::SDK_EXTERNAL_TOOL, "RMCP call_tool failed");
        crate::client::error::ZaiError::ApiError {
            code: codes::SDK_EXTERNAL_TOOL,
            message: "RMCP service error".to_string(),
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
    use futures_util::{StreamExt, TryStreamExt};

    futures_util::stream::iter(calls)
        .map(|(name, arguments)| call_mcp_tool(server, name, arguments))
        .buffered(8)
        .try_collect::<HashMap<_, _>>()
        .await
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

/// Execute tool calls requested by the first choice in a `ChatCompletionResponse`
/// and build tool messages ready to append to the chat.
///
/// This encapsulates:
/// - Extracting tool_calls from the assistant message
/// - Parsing function names and JSON arguments
/// - Executing the RMCP tool via McpToolCaller
/// - Packaging results as TextMessage::tool_with_id
///
/// Returns an empty Vec when there are no tool calls.
/// Malformed calls become in-band error tool messages and are never dispatched
/// to the MCP server. Valid calls run concurrently (up to eight at a time) and
/// their result order matches the request order.
pub async fn execute_tool_calls_as_messages(
    caller: &McpToolCaller,
    resp: &crate::model::chat_base_response::ChatCompletionResponse,
) -> crate::ZaiResult<Vec<crate::model::chat_message_types::TextMessage>> {
    use crate::model::{chat_base_response::ToolCallMessage, chat_message_types::TextMessage};

    let calls: Option<&[ToolCallMessage]> = resp
        .choices()
        .and_then(|v| v.first())
        .and_then(|choice| choice.message())
        .and_then(|message| message.tool_calls());

    let Some(calls) = calls else {
        return Ok(Vec::new());
    };
    debug!(tool_calls = calls.len(), "Dispatching tool calls");

    async fn execute_one(
        caller: &McpToolCaller,
        call: &ToolCallMessage,
    ) -> crate::ZaiResult<TextMessage> {
        let id = call.id();
        let message = |error_type: &str, message: String| {
            let payload = serde_json::json!({
                "error": {"type": error_type, "message": message}
            })
            .to_string();
            text_tool_message(payload, id)
        };
        let Some(function) = call.function() else {
            return Ok(message(
                "missing_function",
                "tool_call.function is missing".to_string(),
            ));
        };
        let name = function.name();
        if name.trim().is_empty() {
            return Ok(message(
                "missing_function_name",
                "tool_call.function.name is blank".to_string(),
            ));
        }
        let arguments = match serde_json::from_str(function.arguments()) {
            Ok(Value::Object(arguments)) => Some(Value::Object(arguments)),
            Ok(_) => {
                return Ok(message(
                    "invalid_arguments",
                    "tool arguments must decode to a JSON object".to_string(),
                ));
            },
            Err(error) => {
                return Ok(message(
                    "invalid_arguments",
                    format!("tool arguments are not valid JSON: {error}"),
                ));
            },
        };
        let (_, payload) = caller.call(name, arguments).await?;
        Ok(text_tool_message(payload.to_string(), id))
    }

    use futures_util::{StreamExt, TryStreamExt};
    futures_util::stream::iter(calls)
        .map(|call| execute_one(caller, call))
        .buffered(8)
        .try_collect()
        .await
}

fn assistant_request_message(
    response: &crate::model::chat_base_response::ChatCompletionResponse,
) -> crate::ZaiResult<Option<crate::model::TextMessage>> {
    use crate::model::{FunctionParams, TextMessage, ToolCall};

    let Some(message) = response
        .choices()
        .and_then(|choices| choices.first())
        .and_then(|choice| choice.message())
    else {
        return Ok(None);
    };
    let Some(calls) = message.tool_calls().filter(|calls| !calls.is_empty()) else {
        return Ok(None);
    };
    let request_calls = calls
        .iter()
        .map(|call| {
            let id = call
                .id()
                .filter(|id| !id.trim().is_empty())
                .ok_or_else(|| validation_error("tool call id must not be blank"))?;
            let function = call
                .function()
                .ok_or_else(|| validation_error("tool call function is required"))?;
            crate::toolkits::core::validate_tool_name(function.name())
                .map_err(|_| validation_error("tool call function name is invalid"))?;
            if !matches!(
                serde_json::from_str(function.arguments()),
                Ok(Value::Object(_))
            ) {
                return Err(validation_error(
                    "tool call arguments must decode to a JSON object",
                ));
            }
            Ok(ToolCall::new_function(
                id,
                FunctionParams::new(function.name(), function.arguments()),
            ))
        })
        .collect::<crate::ZaiResult<Vec<_>>>()?;

    Ok(Some(TextMessage::assistant_with_tools(
        message.content_str().map(str::to_owned),
        request_calls,
    )))
}

/// Perform a complete MCP tool-call roundtrip:
/// - Send the first chat request
/// - Execute any requested tool calls via MCP
/// - Append tool results as tool messages
/// - Disable tools and add an optional system hint
/// - Send the second request and return the final response
///
/// If no tool calls are requested, returns the first response directly.
pub async fn run_mcp_tool_roundtrip<N>(
    caller: &McpToolCaller,
    client: &crate::client::ZaiClient,
    mut chat: crate::model::chat::ChatCompletion<
        N,
        crate::model::chat_message_types::TextMessage,
        crate::model::traits::StreamOff,
    >,
    system_hint_after_tools: Option<&str>,
) -> crate::ZaiResult<crate::model::chat_base_response::ChatCompletionResponse>
where
    N: crate::model::traits::Chat
        + crate::model::traits::ChatToolSupport<Tool = crate::model::tools::Tools>
        + serde::Serialize,
    (N, crate::model::chat_message_types::TextMessage): crate::model::traits::Bounded,
{
    use crate::model::TextMessage;

    let first_resp = chat.send_via(client).await?;
    let Some(assistant_message) = assistant_request_message(&first_resp)? else {
        return Ok(first_resp);
    };

    let tool_msgs: Vec<crate::model::chat_message_types::TextMessage> =
        execute_tool_calls_as_messages(caller, &first_resp).await?;

    chat = chat.add_message(assistant_message);

    for m in tool_msgs {
        chat = chat.add_message(m);
    }

    // Disable tools for the second round to encourage final answer
    chat = chat.clear_tools();

    if let Some(hint) = system_hint_after_tools {
        chat = chat.add_message(TextMessage::system(hint));
    }

    let final_resp = chat.send_via(client).await?;
    Ok(final_resp)
}

/// Extract a concise final text from ChatCompletionResponse when possible.
/// - If content is a string, return it
/// - If content is an array, return the first item of type "text"'s `text`
///   field
/// - Otherwise return None
pub fn extract_final_text(
    resp: &crate::model::chat_base_response::ChatCompletionResponse,
) -> Option<String> {
    let msg = resp.choices()?.first()?.message()?;
    match msg.content() {
        Some(crate::model::chat_base_response::MessageContent::Text(text)) => Some(text.clone()),
        Some(crate::model::chat_base_response::MessageContent::Parts(parts)) => parts
            .iter()
            .find(|part| {
                matches!(
                    part.type_,
                    Some(crate::model::chat_base_response::MessageContentPartType::Text)
                )
            })
            .and_then(|part| part.text.clone()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn response_with_arguments(
        arguments: &str,
    ) -> crate::model::chat_base_response::ChatCompletionResponse {
        serde_json::from_value(serde_json::json!({
            "id": "response-id",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "tool_calls": [{
                        "id": "call-id",
                        "type": "function",
                        "function": {"name": "lookup", "arguments": arguments}
                    }]
                },
                "finish_reason": "tool_calls"
            }]
        }))
        .unwrap()
    }

    #[test]
    fn call_spec_debug_redacts_arguments() {
        let spec = McpCallSpec::new("lookup", Some(serde_json::json!({"token": "secret-value"})));

        let debug = format!("{spec:?}");
        assert!(debug.contains("<redacted>"));
        assert!(!debug.contains("secret-value"));
    }

    #[test]
    fn roundtrip_replays_the_assistant_tool_request() {
        let response = response_with_arguments(r#"{"query":"weather"}"#);
        let message = assistant_request_message(&response).unwrap().unwrap();
        let value = serde_json::to_value(message).unwrap();

        assert_eq!(value["role"], "assistant");
        assert_eq!(value["tool_calls"][0]["id"], "call-id");
        assert_eq!(value["tool_calls"][0]["function"]["name"], "lookup");
    }

    #[test]
    fn roundtrip_rejects_malformed_calls_before_dispatch() {
        let response = response_with_arguments("[]");
        assert!(assistant_request_message(&response).is_err());
    }
}
