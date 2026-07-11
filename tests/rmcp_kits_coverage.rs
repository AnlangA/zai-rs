//! Coverage for toolkits/rmcp_kits.rs (requires --features rmcp-kits).
#![cfg(feature = "rmcp-kits")]

use zai_rs::model::chat_base_response::ChatCompletionResponse;
use zai_rs::toolkits::rmcp_kits::*;

#[test]
fn mcp_call_spec_new() {
    let spec = McpCallSpec::new("tool_name", Some(serde_json::json!({"x": 1})));
    assert_eq!(spec.name, "tool_name");
}

#[test]
fn extract_final_text_from_response() {
    let json_str = r#"{"id":"x","choices":[{"index":0,"message":{"role":"assistant","content":"final answer"},"finish_reason":"stop"}]}"#;
    let resp: ChatCompletionResponse = serde_json::from_str(json_str).unwrap();
    let text = extract_final_text(&resp);
    assert_eq!(text, Some("final answer".to_string()));
}

#[test]
fn extract_final_text_empty_choices() {
    let json_str = r#"{"id":"x","choices":[]}"#;
    let resp: ChatCompletionResponse = serde_json::from_str(json_str).unwrap();
    let text = extract_final_text(&resp);
    assert_eq!(text, None);
}

#[test]
fn extract_final_text_no_message_content() {
    let json_str = r#"{"id":"x","choices":[{"index":0,"message":{"role":"assistant"},"finish_reason":"stop"}]}"#;
    let resp: ChatCompletionResponse = serde_json::from_str(json_str).unwrap();
    let text = extract_final_text(&resp);
    assert_eq!(text, None);
}
