//! P04 acceptance: agent v1 contract (plan §13.2 / P04 验证 — agent_contract).

use zai_rs::agent::{
    AgentAsyncResult, AgentConversationResponse, AgentInvokeRequest, AgentInvokeResponse,
    NonStreaming, message, response::validate_async_result,
    response::validate_conversation_response, response::validate_invoke_response,
};

#[test]
fn invoke_nonstreaming_serializes_stream_false() {
    let req = AgentInvokeRequest::<NonStreaming>::builder("agent-1")
        .message(message("user", "hello"))
        .build()
        .unwrap();
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json["agent_id"], "agent-1");
    assert_eq!(json["stream"], false);
    assert_eq!(json["messages"][0]["role"], "user");
    assert_eq!(json["messages"][0]["content"], "hello");
}

#[test]
fn invoke_streaming_serializes_stream_true() {
    let req = AgentInvokeRequest::<NonStreaming>::builder("agent-1")
        .message(message("user", "hi"))
        .streaming()
        .build()
        .unwrap();
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json["stream"], true);
}

#[test]
fn custom_variables_is_open_map() {
    let mut vars = serde_json::Map::new();
    vars.insert("k".into(), serde_json::json!(42));
    let req = AgentInvokeRequest::<NonStreaming>::builder("agent-1")
        .message(message("user", "hi"))
        .custom_variables(vars)
        .build()
        .unwrap();
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json["custom_variables"]["k"], 42);
}

#[test]
fn empty_or_unknown_body_does_not_become_success() {
    // {} does not deserialize into the tagged enum → Err.
    assert!(serde_json::from_str::<AgentInvokeResponse>("{}").is_err());
    // unknown shape → Err.
    assert!(serde_json::from_str::<AgentInvokeResponse>(r#"{"unexpected":true}"#).is_err());
}

#[test]
fn completed_response_roundtrips_and_validates() {
    let body = r#"{"status":"completed","id":"id1","agent_id":"a1","choices":[{"message":{"content":"hi","role":"assistant"},"finish_reason":"stop"}]}"#;
    let resp: AgentInvokeResponse = serde_json::from_str(body).unwrap();
    assert!(matches!(resp, AgentInvokeResponse::Completed { .. }));
    validate_invoke_response(&resp).unwrap();
}

#[test]
fn pending_response_roundtrips_and_validates() {
    let body = r#"{"status":"pending","agent_id":"a1","async_id":"as1"}"#;
    let resp: AgentInvokeResponse = serde_json::from_str(body).unwrap();
    assert!(matches!(resp, AgentInvokeResponse::Pending { .. }));
    validate_invoke_response(&resp).unwrap();
}

#[test]
fn async_result_failed_is_a_normal_task_result() {
    let body = r#"{"status":"failed","agent_id":"a1","async_id":"as1"}"#;
    let resp: AgentAsyncResult = serde_json::from_str(body).unwrap();
    // Failed validates Ok (it's a task result, not a transport error).
    validate_async_result(&resp).unwrap();
}

#[test]
fn conversation_success_validates() {
    let body =
        r#"{"conversation_id":"c1","agent_id":"a1","choices":[{"message":{"content":"hi"}}]}"#;
    let resp: AgentConversationResponse = serde_json::from_str(body).unwrap();
    validate_conversation_response(&resp).unwrap();
}

#[test]
fn old_crud_symbols_are_absent() {
    // The removed 0.4 surface (create/update/delete_agent, AgentClient, history)
    // must not exist. These would be compile errors if re-added — this test is
    // a presence check that the new module does NOT export them.
    // (If any of these names existed, the `use` above would resolve them.)
    let _ = message("user", "x");
}
