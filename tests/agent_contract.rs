//! Public Agent v1 wire-contract and transport tests.
//!
//! These tests pin the serde contract of the three frozen operations and
//! exercise their production `send_via` paths against a local scripted server.

mod support;

use support::http_server::{ScriptedResponse, TestServer};

use zai_rs::agent::{
    AgentAsyncResult, AgentAsyncResultRequest, AgentAsyncStatus, AgentConversationRequest,
    AgentConversationResponse, AgentConversationVariables, AgentCustomVariables, AgentId,
    AgentInvokeRequest, AgentInvokeResponse, AgentMessage, AgentSlidePage, NonStreaming,
};
use zai_rs::client::{ApiFamily, ZaiClient};

const KEY: &str = "test.12345678901234567890";

async fn agent_server(body: serde_json::Value) -> (TestServer, ZaiClient) {
    let server = TestServer::start(vec![ScriptedResponse::json(200, body)]).await;
    let client = ZaiClient::builder(KEY)
        .allow_insecure_transport(true)
        .endpoint(ApiFamily::AgentV1, format!("{}/api/v1", server.base_url))
        .build()
        .unwrap();
    (server, client)
}

fn assert_agent_request(server: &TestServer, path: &str, expected_body: serde_json::Value) {
    let requests = server.requests();
    assert_eq!(requests.len(), 1, "expected exactly one Agent request");
    let request = &requests[0];
    assert_eq!(request.method, "POST");
    assert_eq!(request.path, path);
    let expected_authorization = format!("Bearer {KEY}");
    assert_eq!(
        request.authorization.as_deref(),
        Some(expected_authorization.as_str())
    );
    assert!(
        request.headers.iter().any(|(name, value)| {
            name == "content-type" && value.starts_with("application/json")
        })
    );
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&request.body).unwrap(),
        expected_body
    );
}

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

#[test]
fn invoke_response_accepts_additive_provider_fields() {
    let response: AgentInvokeResponse = serde_json::from_value(serde_json::json!({
        "id": "invoke-1",
        "agent_id": "general_translation",
        "choices": [{
            "index": 0,
            "messages": [{
                "role": "assistant",
                "content": {
                    "type": "text",
                    "text": "done",
                    "provider_content_metadata": {"confidence": 0.99}
                },
                "provider_message_metadata": "future"
            }],
            "finish_reason": "stop",
            "provider_choice_metadata": true
        }],
        "usage": {
            "prompt_tokens": 3,
            "completion_tokens": 2,
            "total_tokens": 5,
            "cached_tokens": 1
        },
        "provider_trace_id": "trace-1"
    }))
    .unwrap();

    assert!(matches!(
        response,
        AgentInvokeResponse::Completed(ref completed)
            if completed.id == "invoke-1" && completed.choices.len() == 1
    ));
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
        serde_json::json!({
            "agent_id": "a",
            "provider_state": "future"
        }),
        serde_json::json!({
            "id": "invoke-1",
            "agent_id": "a",
            "choices": [{
                "messages": [{
                    "content": {"type": "future_content", "value": "unsupported"}
                }]
            }]
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
    assert!(
        serde_json::from_value::<AgentAsyncResult>(serde_json::json!({
            "agent_id": "a",
            "async_id": "t",
            "status": "pending",
            "choices": [{"messages": []}]
        }))
        .is_err()
    );
}

#[test]
fn async_result_response_accepts_additive_provider_fields() {
    let response: AgentAsyncResult = serde_json::from_value(serde_json::json!({
        "agent_id": "general_translation",
        "async_id": "task-1",
        "status": "success",
        "choices": [{
            "messages": [{
                "role": "assistant",
                "content": [{
                    "type": "file_url",
                    "file_url": "https://example.test/result.txt",
                    "provider_content_metadata": {"expires_in": 300}
                }],
                "provider_message_metadata": "future"
            }],
            "provider_choice_metadata": true
        }],
        "usage": {
            "total_tokens": 5,
            "cached_tokens": 1
        },
        "provider_trace_id": "trace-1"
    }))
    .unwrap();

    assert!(matches!(
        response,
        AgentAsyncResult::Success { ref choices, .. } if choices.len() == 1
    ));
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
    assert!(
        serde_json::from_value::<AgentConversationResponse>(serde_json::json!({
            "conversation_id": "c",
            "agent_id": "a",
            "choices": [{"message": []}],
            "error": {"code": "failed"}
        }))
        .is_err()
    );
}

#[test]
fn conversation_response_accepts_additive_provider_fields() {
    let response: AgentConversationResponse = serde_json::from_value(serde_json::json!({
        "conversation_id": "conversation-1",
        "agent_id": "slides_glm_agent",
        "choices": [{
            "message": [{
                "role": "assistant",
                "content": [{
                    "type": "file_url",
                    "file_url": "https://example.test/slides.pptx",
                    "provider_content_metadata": {"expires_in": 300}
                }],
                "provider_message_metadata": "future"
            }],
            "provider_choice_metadata": true
        }],
        "provider_trace_id": "trace-1"
    }))
    .unwrap();

    assert!(matches!(
        response,
        AgentConversationResponse::Success(ref success) if success.choices.len() == 1
    ));
}

#[tokio::test]
async fn invoke_send_via_uses_agent_route_auth_and_json_contract() {
    let (server, client) = agent_server(serde_json::json!({
        "id": "invoke-1",
        "agent_id": "general_translation",
        "choices": [{
            "index": 0,
            "messages": [{"role": "assistant", "content": "done"}],
            "finish_reason": "stop"
        }]
    }))
    .await;
    let mut variables = AgentCustomVariables::new();
    variables.insert("target_lang", serde_json::json!("en"));
    let request = AgentInvokeRequest::<NonStreaming>::builder(AgentId::GeneralTranslation)
        .message(AgentMessage::user("hello"))
        .custom_variables(variables)
        .build()
        .unwrap();

    let response = request.send_via(&client).await.unwrap();

    assert!(matches!(
        response,
        AgentInvokeResponse::Completed(ref completed) if completed.id == "invoke-1"
    ));
    assert_agent_request(
        &server,
        "/api/v1/agents",
        serde_json::json!({
            "agent_id": "general_translation",
            "stream": false,
            "messages": [{"role": "user", "content": "hello"}],
            "custom_variables": {"target_lang": "en"}
        }),
    );
    server.shutdown().await;
}

#[tokio::test]
async fn async_result_send_via_uses_agent_poll_route_and_validates_response() {
    let (server, client) = agent_server(serde_json::json!({
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
    .await;
    let request = AgentAsyncResultRequest::new("general_translation", "task-1").unwrap();

    let response = request.send_via(&client).await.unwrap();

    assert_eq!(response.status(), AgentAsyncStatus::Success);
    assert_agent_request(
        &server,
        "/api/v1/agents/async-result",
        serde_json::json!({
            "async_id": "task-1",
            "agent_id": "general_translation"
        }),
    );
    server.shutdown().await;
}

#[tokio::test]
async fn conversation_send_via_uses_agent_conversation_route_and_validates_response() {
    let (server, client) = agent_server(serde_json::json!({
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
    .await;
    let variables = AgentConversationVariables::new()
        .with_include_pdf(true)
        .with_pages([AgentSlidePage::new(1.0, 25.4, 14.29)]);
    let request = AgentConversationRequest::new("slides_glm_agent", "conversation-1")
        .unwrap()
        .with_custom_variables(variables);

    let response = request.send_via(&client).await.unwrap();

    assert!(matches!(
        response,
        AgentConversationResponse::Success(ref success)
            if success.conversation_id == "conversation-1"
    ));
    assert_agent_request(
        &server,
        "/api/v1/agents/conversation",
        serde_json::json!({
            "agent_id": "slides_glm_agent",
            "conversation_id": "conversation-1",
            "custom_variables": {
                "include_pdf": true,
                "pages": [{"position": 1.0, "width": 25.4, "height": 14.29}]
            }
        }),
    );
    server.shutdown().await;
}
