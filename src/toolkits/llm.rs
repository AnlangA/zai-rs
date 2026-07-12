//! JSON parsing helpers for LLM tool_calls responses.
//!
//! These utilities help extract tool call requests from LLM responses that
//! follow OpenAI/Zhipu-style schemas where tool calls are returned under
//! `choices[*].message.tool_calls`.

use serde_json::Value;

/// A parsed tool call request from an LLM response.
#[derive(Debug, Clone, PartialEq)]
pub struct LlmToolCall<'a> {
    /// Tool-call id (used to correlate the tool's reply with the request).
    pub id: &'a str,
    /// Name of the function/tool to invoke.
    pub name: &'a str,
    /// Raw string form of arguments if the provider returned it as a JSON
    /// string. Useful for diagnostics; may be None if provider already
    /// returned an object.
    pub arguments_raw: Option<&'a str>,
    /// Parsed JSON arguments. For providers that return a string, we attempt to
    /// parse it. If parsing fails, we return the raw string in this field.
    pub arguments: Value,
}

/// Parse tool calls from either a direct `tool_calls` object or every choice
/// in a full response.
pub fn parse_tool_calls(response: &Value) -> Vec<LlmToolCall<'_>> {
    let mut results = Vec::new();

    if let Some(calls) = response.get("tool_calls").and_then(|v| v.as_array()) {
        results.extend(parse_tool_calls_array(calls));
    } else if let Some(choices) = response.get("choices").and_then(|v| v.as_array()) {
        for choice in choices {
            if let Some(msg) = choice.get("message")
                && let Some(calls) = msg.get("tool_calls").and_then(|v| v.as_array())
            {
                results.extend(parse_tool_calls_array(calls));
            }
        }
    }

    results
}

/// Parse a `tool_calls` array, skipping entries without an id or function name.
fn parse_tool_calls_array(calls: &[Value]) -> Vec<LlmToolCall<'_>> {
    calls.iter().filter_map(parse_tool_call).collect()
}

/// Parse all well-formed tool calls from a single assistant message object.
pub fn parse_tool_calls_from_message(message: &Value) -> Vec<LlmToolCall<'_>> {
    let Some(calls) = message.get("tool_calls").and_then(|v| v.as_array()) else {
        return Vec::new();
    };
    parse_tool_calls_array(calls)
}

fn parse_tool_call(tool_call: &Value) -> Option<LlmToolCall<'_>> {
    let id = tool_call.get("id")?.as_str()?;
    let function = tool_call.get("function")?.as_object()?;
    let name = function.get("name")?.as_str()?;
    let (arguments_raw, arguments) = match function.get("arguments") {
        Some(Value::String(raw)) => (
            Some(raw.as_str()),
            serde_json::from_str(raw).unwrap_or_else(|_| Value::String(raw.clone())),
        ),
        Some(value) => (None, value.clone()),
        None => (None, Value::Null),
    };
    Some(LlmToolCall {
        id,
        name,
        arguments_raw,
        arguments,
    })
}

/// Convenience: parse the first tool call found in the response.
pub fn parse_first_tool_call(response: &Value) -> Option<LlmToolCall<'_>> {
    parse_tool_calls(response).into_iter().next()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn test_parse_tool_calls_from_message() {
        let message = json!({
            "tool_calls": [
                {
                    "id": "call_123",
                    "type": "function",
                    "function": {
                        "name": "get_weather",
                        "arguments": "{\"city\": \"Shenzhen\"}"
                    }
                }
            ]
        });

        let calls = parse_tool_calls_from_message(&message);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].id, "call_123");
        assert_eq!(calls[0].name, "get_weather");
    }

    #[test]
    fn test_parse_tool_calls_from_message_object_args() {
        let message = json!({
            "tool_calls": [
                {
                    "id": "call_456",
                    "type": "function",
                    "function": {
                        "name": "calc",
                        "arguments": {"op": "add", "a": 1, "b": 2}
                    }
                }
            ]
        });

        let calls = parse_tool_calls_from_message(&message);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "calc");
        assert_eq!(calls[0].arguments, json!({"op": "add", "a": 1, "b": 2}));
    }

    #[test]
    fn test_parse_tool_calls_from_response() {
        let response = json!({
            "choices": [
                {
                    "message": {
                        "tool_calls": [
                            {
                                "id": "call_789",
                                "function": {
                                    "name": "test_tool",
                                    "arguments": "{\"input\": \"test\"}"
                                }
                            }
                        ]
                    }
                }
            ]
        });

        let calls = parse_tool_calls(&response);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "test_tool");
    }

    #[test]
    fn test_parse_first_tool_call() {
        let response = json!({
            "choices": [
                {
                    "message": {
                        "tool_calls": [
                            {
                                "id": "call_first",
                                "function": {
                                    "name": "first_tool",
                                    "arguments": "{}"
                                }
                            },
                            {
                                "id": "call_second",
                                "function": {
                                    "name": "second_tool",
                                    "arguments": "{}"
                                }
                            }
                        ]
                    }
                }
            ]
        });

        let first = parse_first_tool_call(&response);
        assert!(first.is_some());
        assert_eq!(first.unwrap().name, "first_tool");
    }

    #[test]
    fn test_parse_tool_calls_direct() {
        let response = json!({
            "tool_calls": [
                {
                    "id": "call_direct",
                    "function": {
                        "name": "direct_tool",
                        "arguments": "{}"
                    }
                }
            ]
        });

        let calls = parse_tool_calls(&response);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "direct_tool");
    }

    #[test]
    fn test_parse_multiple_tool_calls() {
        let message = json!({
            "tool_calls": [
                {
                    "id": "call_1",
                    "function": {
                        "name": "tool_a",
                        "arguments": "{\"x\": 1}"
                    }
                },
                {
                    "id": "call_2",
                    "function": {
                        "name": "tool_b",
                        "arguments": "{\"y\": 2}"
                    }
                }
            ]
        });

        let calls = parse_tool_calls_from_message(&message);
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].name, "tool_a");
        assert_eq!(calls[1].name, "tool_b");
    }
}
