//! Behavioral tests for the optional RMCP tool bridge.

#![cfg(feature = "rmcp-kits")]

use zai_rs::model::chat_base_response::ChatCompletionResponse;
use zai_rs::toolkits::rmcp_kits::{McpCallSpec, extract_final_text};

#[test]
fn call_spec_validates_object_arguments() {
    let arguments = serde_json::json!({"x": 1});
    let spec = McpCallSpec::new("tool_name", Some(arguments.clone()));
    assert_eq!(spec.name, "tool_name");
    assert_eq!(spec.arguments, Some(arguments));
    spec.validate().unwrap();

    assert!(
        McpCallSpec::new("tool_name", Some(serde_json::json!(1)))
            .validate()
            .is_err()
    );
    assert!(McpCallSpec::new("   ", None).validate().is_err());
    assert!(McpCallSpec::new("bad\nname", None).validate().is_err());
    assert!(McpCallSpec::new("x".repeat(65), None).validate().is_err());
    assert!(McpCallSpec::new("valid-tool_name", None).validate().is_ok());
}

#[test]
fn extract_final_text_from_response() {
    let json = r#"{"id":"x","choices":[{"index":0,"message":{"role":"assistant","content":"final answer"},"finish_reason":"stop"}]}"#;
    let response: ChatCompletionResponse = serde_json::from_str(json).unwrap();
    assert_eq!(
        extract_final_text(&response).as_deref(),
        Some("final answer")
    );
}

#[test]
fn extract_final_text_handles_missing_content() {
    for json in [
        r#"{"id":"x","choices":[]}"#,
        r#"{"id":"x","choices":[{"index":0,"message":{"role":"assistant"},"finish_reason":"stop"}]}"#,
    ] {
        let response: ChatCompletionResponse = serde_json::from_str(json).unwrap();
        assert_eq!(extract_final_text(&response), None);
    }
}
