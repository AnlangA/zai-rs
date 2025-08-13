use serde_json::{self, Value};

use zai_rs::model::{ChatMessage, ToolType};

// Generic helper to assert a type implements Serialize without constructing a value.
fn assert_serialize<T: serde::Serialize>() {}

#[test]
fn chat_message_serialize_fails_when_all_none() {
    // ChatMessage must have at least one inner message; all None should error.
    let msg = ChatMessage {
        user_message: None,
        assistant_message: None,
        system_message: None,
        tool_message: None,
    };

    let err = serde_json::to_string(&msg)
        .expect_err("serializing ChatMessage with all fields None should produce an error");

    let msg = err.to_string();
    assert!(
        msg.contains("ChatMessage must have at least one non-none message"),
        "unexpected error: {msg}"
    );
}

#[test]
fn tool_type_serializes_in_snake_case() {
    // ToolType variants should serialize to snake_case strings.
    let f = serde_json::to_string(&ToolType::Function).unwrap();
    let w = serde_json::to_string(&ToolType::WebSearch).unwrap();
    let r = serde_json::to_string(&ToolType::Retrieval).unwrap();

    assert_eq!(f, "\"function\"");
    assert_eq!(w, "\"web_search\"");
    assert_eq!(r, "\"retrieval\"");
}

#[test]
fn type_level_assertions_for_other_serializers() {
    // These assert that the types implement Serialize (compile-time), without constructing values.
    // Constructing ToolMessage/ToolCall here is not possible due to private fields; behavior tests
    // for those types should be covered by unit tests within the crate or by exposing constructors.
    assert_serialize::<zai_rs::model::ToolMessage>();
    assert_serialize::<zai_rs::model::ToolCall>();
}

// The following are illustrative tests demonstrating desired behaviors for ToolMessage/ToolCall.
// They are gated behind a cfg that is always false so they don't participate in compilation.
// Once the crate exposes constructors/builders for these types, remove the cfg and make them real tests.
#[cfg(any())]
mod future_behavior_tests {
    use super::*;
    use zai_rs::model::{FunctionCall, ToolCall, ToolMessage, ToolType};

    #[test]
    fn tool_message_requires_content_or_tool_calls() {
        // Example (won't compile until constructors are exposed):
        let tm = ToolMessage::new("assistant".into(), None, None);
        let err = serde_json::to_string(&tm)
            .expect_err("serializing ToolMessage with neither content nor tool_calls should error");
        assert!(
            err.to_string()
                .contains("ToolMessage must have at least one of 'content' or 'tool_calls'")
        );
    }

    #[test]
    fn tool_call_requires_function_when_type_is_function() {
        // Missing `function` should error when type is Function:
        let tc = ToolCall::new("id1".into(), ToolType::Function, None);
        let err = serde_json::to_string(&tc).expect_err(
            "serializing ToolCall with type=function but without function should error",
        );
        assert!(
            err.to_string()
                .contains("ToolCall.function must be present when type is 'function'")
        );

        // When present, it should serialize with `function` field:
        let tc_ok = ToolCall::new(
            "id2".into(),
            ToolType::Function,
            Some(FunctionCall::new(
                "search".into(),
                "{\"q\":\"rust\"}".into(),
            )),
        );
        let v: Value = serde_json::to_value(&tc_ok).unwrap();
        assert_eq!(v["id"], "id2");
        assert_eq!(v["type"], "function");
        assert!(v.get("function").is_some());

        // For non-function types, `function` must not appear:
        let tc_ws = ToolCall::new("id3".into(), ToolType::WebSearch, None);
        let v: Value = serde_json::to_value(&tc_ws).unwrap();
        assert_eq!(v["id"], "id3");
        assert_eq!(v["type"], "web_search");
        assert!(v.get("function").is_none());
    }
}
