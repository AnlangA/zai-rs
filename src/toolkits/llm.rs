//! JSON parsing helpers for LLM tool_calls responses.
//!
//! These utilities help extract tool call requests from LLM responses that follow
//! OpenAI/Zhipu-style schemas where tool calls are returned under
//! `choices[*].message.tool_calls`.

use serde_json::Value;
use std::borrow::Cow;

/// A parsed tool call request from an LLM response with zero-copy optimization.
#[derive(Debug, Clone, PartialEq)]
pub struct LlmToolCall<'a> {
    pub id: Cow<'a, str>,
    pub name: Cow<'a, str>,
    /// Raw string form of arguments if the provider returned it as a JSON string.
    /// Useful for diagnostics; may be None if provider already returned an object.
    pub arguments_raw: Option<Cow<'a, str>>,
    /// Parsed JSON arguments. For providers that return a string, we attempt to
    /// parse it. If parsing fails, we return the raw string in this field.
    pub arguments: Value,
}

/// Normalize JSON arguments for better consistency
pub fn normalize_arguments(args: &Value) -> Value {
    match args {
        Value::String(s) => {
            // Try to parse string as JSON
            serde_json::from_str(s).unwrap_or_else(|_| Value::String(s.clone()))
        }
        Value::Object(obj) => {
            // Normalize object keys and values
            let mut normalized = serde_json::Map::new();
            for (k, v) in obj {
                let normalized_key = k.trim().to_lowercase();
                normalized.insert(normalized_key, normalize_arguments(v));
            }
            Value::Object(normalized)
        }
        _ => args.clone(),
    }
}

/// Parse tool calls with better error recovery and zero-copy optimization
pub fn parse_tool_calls_robust(response: &Value) -> Vec<LlmToolCall<'_>> {
    let mut results = Vec::new();

    // Try multiple parsing strategies
    if let Some(calls) = response.get("tool_calls").and_then(|v| v.as_array()) {
        // Direct tool_calls
        results.extend(parse_tool_calls_array(calls));
    } else if let Some(choices) = response.get("choices").and_then(|v| v.as_array()) {
        // Choices format
        for choice in choices {
            if let Some(msg) = choice.get("message")
                && let Some(calls) = msg.get("tool_calls").and_then(|v| v.as_array()) {
                    results.extend(parse_tool_calls_array(calls));
                }
        }
    } else if let Some(function_call) = response.get("function_call") {
        // Legacy function_call format
        if let Some(call) = parse_legacy_function_call(function_call) {
            results.push(call);
        }
    }

    results
}

/// Parse legacy function_call format
fn parse_legacy_function_call(function_call: &Value) -> Option<LlmToolCall<'static>> {
    let name = function_call.get("name")?.as_str()?;

    let arguments = function_call
        .get("arguments")
        .cloned()
        .unwrap_or(Value::Null);

    let (arguments_raw, parsed_args) = match arguments {
        Value::String(s) => {
            let parsed = match serde_json::from_str::<Value>(&s) {
                Ok(v) => v,
                Err(_) => Value::String(s.clone()),
            };
            (Some(Cow::Owned(s)), parsed)
        }
        other => (None, other),
    };

    Some(LlmToolCall {
        id: Cow::Borrowed("legacy"),
        name: Cow::Owned(name.to_string()),
        arguments_raw,
        arguments: parsed_args,
    })
}

/// Parse tool calls array with zero-copy optimization
fn parse_tool_calls_array(calls: &[Value]) -> Vec<LlmToolCall<'_>> {
    let mut out = Vec::new();

    for tc in calls {
        let Some(id) = tc.get("id").and_then(|v| v.as_str()) else {
            continue;
        };
        let Some(func) = tc.get("function").and_then(|v| v.as_object()) else {
            continue;
        };
        let Some(name) = func.get("name").and_then(|v| v.as_str()) else {
            continue;
        };

        let (arguments_raw, arguments) = match func.get("arguments") {
            Some(Value::String(s)) => {
                // Provider returned stringified JSON; try to parse.
                match serde_json::from_str::<Value>(s) {
                    Ok(v) => (Some(Cow::Borrowed(s.as_str())), v),
                    Err(_) => (Some(Cow::Borrowed(s.as_str())), Value::String(s.clone())),
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
            id: Cow::Borrowed(id),
            name: Cow::Owned(name.to_string()),
            arguments_raw,
            arguments,
        });
    }

    out
}

/// Parse all tool calls from a single assistant message object with zero-copy optimization.
pub fn parse_tool_calls_from_message(message: &Value) -> Vec<LlmToolCall<'_>> {
    let mut out = Vec::new();
    let Some(calls) = message.get("tool_calls").and_then(|v| v.as_array()) else {
        return out;
    };

    for tc in calls {
        let Some(id) = tc.get("id").and_then(|v| v.as_str()) else {
            continue;
        };
        let Some(func) = tc.get("function").and_then(|v| v.as_object()) else {
            continue;
        };
        let Some(name) = func.get("name").and_then(|v| v.as_str()) else {
            continue;
        };

        let (arguments_raw, arguments) = match func.get("arguments") {
            Some(Value::String(s)) => {
                // Provider returned stringified JSON; try to parse.
                match serde_json::from_str::<Value>(s) {
                    Ok(v) => (Some(Cow::Borrowed(s.as_str())), v),
                    Err(_) => (Some(Cow::Borrowed(s.as_str())), Value::String(s.clone())),
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
            id: Cow::Borrowed(id),
            name: Cow::Owned(name.to_string()),
            arguments_raw,
            arguments,
        });
    }

    out
}

/// Parse tool calls from a full LLM response object with zero-copy optimization.
/// This aggregates tool calls from all choices' messages.
pub fn parse_tool_calls(response: &Value) -> Vec<LlmToolCall<'_>> {
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
pub fn parse_first_tool_call(response: &Value) -> Option<LlmToolCall<'_>> {
    parse_tool_calls(response).into_iter().next()
}
