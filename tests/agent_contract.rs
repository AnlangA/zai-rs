//! Public Agent v1 wire-contract tests not covered by module unit tests.
//!
//! Agent v1 currently models only the frozen wire contracts: `src/agent`
//! intentionally contains no network facade and no `send_via` path, so these
//! tests pin the serde contract of the three frozen operations
//! (`agents.invoke`, `agents.async-result`, `agents.conversation`) against
//! their documented success invariants instead of exercising HTTP transport.

use zai_rs::agent::{
    AgentAsyncResult, AgentAsyncResultRequest, AgentAsyncStatus, AgentConversationRequest,
    AgentConversationResponse, AgentConversationVariables, AgentCustomVariables, AgentId,
    AgentInvokeRequest, AgentInvokeResponse, AgentMessage, AgentSlidePage, NonStreaming,
};

#[test]
fn custom_variables_is_open_map() {
    let mut variables = AgentCustomVariables::new();
    variables.insert("k", serde_json::json!(42));
    let request = AgentInvokeRequest::<NonStreaming>::builder(AgentId::GeneralTranslation)
        .message(AgentMessage::user("hi"))
        .custom_variables(variables)
        .build()
        .unwrap();
    let json = serde_json::to_value(request).unwrap();
    assert_eq!(json["custom_variables"]["k"], 42);
}

/// `agents.invoke` Completed invariant: id + agent_id + non-empty choices.
#[test]
fn invoke_completed_response_satisfies_the_frozen_invariant() {
    let response: AgentInvokeResponse = serde_json::from_value(serde_json::json!({
        "id": "invoke-1",
        "agent_id": "general_translation",
        "conversation_id": "conversation-1",
        "choices": [{
            "index": 0,
            "messages": [{"role": "assistant", "content": "done"}],
            "finish_reason": "stop"
        }],
        "usage": {"prompt_tokens": 3, "completion_tokens": 2, "total_tokens": 5}
    }))
    .unwrap();

    let AgentInvokeResponse::Completed(completed) = &response else {
        panic!("expected a completed invocation");
    };
    assert_eq!(completed.id, "invoke-1");
    assert_eq!(completed.agent_id, "general_translation");
    assert!(!completed.choices.is_empty());
    response.validate().unwrap();
}

/// `agents.invoke` Pending invariant: agent_id + async_id (the frozen schema
/// has no `status` property on the wire).
#[test]
fn invoke_pending_response_satisfies_the_frozen_invariant() {
    let response: AgentInvokeResponse = serde_json::from_value(serde_json::json!({
        "agent_id": "general_translation",
        "async_id": "task-1"
    }))
    .unwrap();

    let AgentInvokeResponse::Pending(pending) = &response else {
        panic!("expected a pending invocation");
    };
    assert_eq!(pending.agent_id, "general_translation");
    assert_eq!(pending.async_id, "task-1");
    response.validate().unwrap();
}

/// Payloads that break the `agents.invoke` invariant are rejected at decode
/// time: empty choices fail the Completed shape, and mixing completed with
/// pending fields is contradictory.
#[test]
fn invoke_response_rejects_invariant_violations() {
    for value in [
        serde_json::json!({"id": "invoke-1", "agent_id": "a", "choices": []}),
        serde_json::json!({
            "id": "invoke-1",
            "agent_id": "a",
            "async_id": "task-1",
            "choices": [{"index": 0}]
        }),
    ] {
        assert!(serde_json::from_value::<AgentInvokeResponse>(value).is_err());
    }
}

/// `agents.async-result`: the request body is exactly `{async_id, agent_id}`
/// and the response keeps the Pending / Succeeded invariant.
#[test]
fn async_result_contract_covers_pending_and_succeeded_shapes() {
    let request = AgentAsyncResultRequest::new("general_translation", "task-1").unwrap();
    assert_eq!(
        serde_json::to_value(&request).unwrap(),
        serde_json::json!({"async_id": "task-1", "agent_id": "general_translation"})
    );

    // Pending: agent_id + async_id.
    let pending: AgentAsyncResult = serde_json::from_value(serde_json::json!({
        "agent_id": "general_translation",
        "async_id": "task-1",
        "status": "pending"
    }))
    .unwrap();
    assert_eq!(pending.status(), AgentAsyncStatus::Pending);
    pending.validate().unwrap();

    // Succeeded: agent_id + async_id + non-empty choices.
    let succeeded: AgentAsyncResult = serde_json::from_value(serde_json::json!({
        "agent_id": "general_translation",
        "async_id": "task-1",
        "status": "success",
        "choices": [{
            "messages": [{
                "role": "assistant",
                "content": [{
                    "type": "file_url",
                    "file_url": "https://example.test/result.txt"
                }]
            }]
        }]
    }))
    .unwrap();
    let AgentAsyncResult::Success { choices, .. } = &succeeded else {
        panic!("expected a succeeded async result");
    };
    assert!(!choices.is_empty());
    succeeded.validate().unwrap();

    // A success payload without choices violates the invariant.
    assert!(
        serde_json::from_value::<AgentAsyncResult>(serde_json::json!({
            "agent_id": "a",
            "async_id": "t",
            "status": "success",
            "choices": []
        }))
        .is_err()
    );
}

/// `agents.conversation`: the request carries the two identifiers plus the
/// closed custom-variables object, and a success keeps
/// conversation_id + agent_id + non-empty choices.
#[test]
fn conversation_contract_covers_request_and_success_shapes() {
    let variables = AgentConversationVariables::new()
        .with_include_pdf(true)
        .with_pages([AgentSlidePage::new(1.0, 25.4, 14.29)]);
    let request = AgentConversationRequest::new("slides_glm_agent", "conversation-1")
        .unwrap()
        .with_custom_variables(variables);
    assert_eq!(
        serde_json::to_value(&request).unwrap(),
        serde_json::json!({
            "agent_id": "slides_glm_agent",
            "conversation_id": "conversation-1",
            "custom_variables": {
                "include_pdf": true,
                "pages": [{"position": 1.0, "width": 25.4, "height": 14.29}]
            }
        })
    );

    // The frozen schema uses the singular `message` field for the message array.
    let response: AgentConversationResponse = serde_json::from_value(serde_json::json!({
        "conversation_id": "conversation-1",
        "agent_id": "slides_glm_agent",
        "choices": [{
            "message": [{
                "role": "assistant",
                "content": [{
                    "type": "file_url",
                    "file_url": "https://example.test/slides.pptx"
                }]
            }]
        }]
    }))
    .unwrap();
    let AgentConversationResponse::Success(success) = &response else {
        panic!("expected a successful conversation response");
    };
    assert_eq!(success.conversation_id, "conversation-1");
    assert_eq!(success.agent_id, "slides_glm_agent");
    assert!(!success.choices.is_empty());
    response.validate().unwrap();

    // Empty choices violate the invariant.
    assert!(
        serde_json::from_value::<AgentConversationResponse>(serde_json::json!({
            "conversation_id": "c",
            "agent_id": "a",
            "choices": []
        }))
        .is_err()
    );
}
