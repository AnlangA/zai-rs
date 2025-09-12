//! JSON parsing helpers for LLM tool_calls responses.
//!
//! These utilities help extract tool call requests from LLM responses that follow
//! OpenAI/Zhipu-style schemas where tool calls are returned under
//! `choices[*].message.tool_calls`.

use serde_json::Value;

/// A parsed tool call request from an LLM response.
#[derive(Debug, Clone, PartialEq)]
pub struct LlmToolCall {
    pub id: String,
    pub name: String,
    /// Raw string form of arguments if the provider returned it as a JSON string.
    /// Useful for diagnostics; may be None if provider already returned an object.
    pub arguments_raw: Option<String>,
    /// Parsed JSON arguments. For providers that return a string, we attempt to
    /// parse it. If parsing fails, we return the raw string in this field.
    pub arguments: Value,
}

/// Parse all tool calls from a single assistant message object.
///
/// The `message` should be a JSON object containing an optional `tool_calls: []` array,
/// with each entry shaped like `{ id, type, function: { name, arguments } }`.
pub fn parse_tool_calls_from_message(message: &Value) -> Vec<LlmToolCall> {
    let mut out = Vec::new();
    let Some(calls) = message.get("tool_calls").and_then(|v| v.as_array()) else {
        return out;
    };

    for tc in calls {
        let Some(id) = tc.get("id").and_then(|v| v.as_str()).map(|s| s.to_string()) else {
            continue;
        };
        let Some(func) = tc.get("function").and_then(|v| v.as_object()) else {
            continue;
        };
        let Some(name) = func
            .get("name")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
        else {
            continue;
        };

        let (arguments_raw, arguments) = match func.get("arguments") {
            Some(Value::String(s)) => {
                // Provider returned stringified JSON; try to parse.
                match serde_json::from_str::<Value>(s) {
                    Ok(v) => (Some(s.clone()), v),
                    Err(_) => (Some(s.clone()), Value::String(s.clone())),
                }
            }
            Some(v @ Value::Object(_)) | Some(v @ Value::Array(_)) => (None, v.clone()),
            Some(v) => {
                // Unexpected primitive; keep as-is for robustness.
                (None, v.clone())
            }
            None => (None, Value::Null),
        };

        out.push(LlmToolCall {
            id,
            name,
            arguments_raw,
            arguments,
        });
    }

    out
}

/// Parse tool calls from a full LLM response object.
/// This aggregates tool calls from all choices' messages.
pub fn parse_tool_calls(response: &Value) -> Vec<LlmToolCall> {
    let mut all = Vec::new();
    if let Some(choices) = response.get("choices").and_then(|v| v.as_array()) {
        for ch in choices {
            if let Some(msg) = ch.get("message") {
                all.extend(parse_tool_calls_from_message(msg));
            }
        }
    }
    all
}

/// Convenience: parse the first tool call found in the response.
pub fn parse_first_tool_call(response: &Value) -> Option<LlmToolCall> {
    parse_tool_calls(response).into_iter().next()
}
