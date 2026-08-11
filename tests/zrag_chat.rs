//! External-crate contract tests for stream-only ZRAG agent chat.

mod support;

use serde_json::{Value, json};
use support::http_server::{ScriptedResponse, TestServer};
use zai_rs::{
    ZaiClient,
    client::{ApiFamily, error::codes},
    zrag::{
        AgentStreamEvent, ZragChatContentPart, ZragChatMessage, ZragChatRequest, ZragChatRetrieval,
    },
};

const KEY: &str = "test.12345678901234567890";

fn client_for(server: &TestServer) -> ZaiClient {
    ZaiClient::builder(KEY)
        .allow_insecure_transport(true)
        .endpoint(ApiFamily::Zrag, format!("{}/api/zrag", server.base_url))
        .build()
        .unwrap()
}

fn request() -> ZragChatRequest {
    ZragChatRequest::new(
        vec![ZragChatMessage::user(vec![
            ZragChatContentPart::text("question"),
            ZragChatContentPart::image_url("https://example.test/image.png"),
        ])],
        ZragChatRetrieval::new(vec!["knowledge-1".to_string()])
            .with_top_k(8)
            .with_top_n(10)
            .with_reranking(false)
            .with_similarity_threshold(0.2),
    )
    .with_model("glm-5v-turbo")
    .with_temperature(0.2)
    .with_max_steps(10)
    .with_thinking(true)
}

fn request_header<'a>(
    request: &'a support::http_server::CapturedRequest,
    name: &str,
) -> Option<&'a str> {
    request
        .headers
        .iter()
        .find(|(header, _)| header.eq_ignore_ascii_case(name))
        .map(|(_, value)| value.as_str())
}

#[test]
fn session_header_is_not_available_as_a_global_additional_header() {
    assert!(
        zai_rs::client::AdditionalHeader::new("X-Session-Id", "must-be-operation-local").is_err()
    );
}

#[tokio::test]
async fn session_header_is_scoped_to_one_operation() {
    let done = || ScriptedResponse::raw(200, "text/event-stream", "data: {\"type\":\"done\"}\n\n");
    let server = TestServer::start(vec![done(), done()]).await;
    let client = client_for(&server);

    let mut continued = request()
        .with_session_id("one-operation-only")
        .stream_via(&client)
        .await
        .unwrap();
    assert!(continued.next().await.unwrap().unwrap().is_done());

    let mut fresh = request().stream_via(&client).await.unwrap();
    assert!(fresh.next().await.unwrap().unwrap().is_done());

    let captured = server.requests();
    assert_eq!(captured.len(), 2);
    assert_eq!(
        request_header(&captured[0], "x-session-id"),
        Some("one-operation-only")
    );
    assert_eq!(request_header(&captured[1], "x-session-id"), None);
}

#[tokio::test]
async fn public_request_sends_operation_header_and_yields_typed_done() {
    let body = concat!(
        "data: {\"type\":\"session_created\",\"sessionId\":\"server-session\"}\n\n",
        "data: {\"type\":\"reasoning\",\"data\":\"reasoning text\"}\n\n",
        "data: {\"type\":\"thought\",\"data\":\"thought text\"}\n\n",
        "data: {\"type\":\"tool_call\",\"data\":{\"callId\":\"call-1\",\"toolName\":\"retrieve\",\"arguments\":{\"q\":\"x\"}}}\n\n",
        "data: {\"type\":\"tool_result\",\"data\":{\"callId\":\"call-1\",\"toolName\":\"retrieve\",\"result\":{\"found\":true},\"status\":\"future_status\",\"durationMs\":4}}\n\n",
        "data: {\"type\":\"answer\",\"data\":\"answer text\"}\n\n",
        "data: {\"type\":\"done\",\"messageId\":\"message-1\",\"usage\":{\"prompt_tokens\":2,\"completion_tokens\":3,\"total_tokens\":5,\"total_calls\":1}}\n\n",
        "data: {\"type\":\"answer\",\"data\":\"must not be yielded\"}\n\n",
    );
    let server = TestServer::start(vec![ScriptedResponse::raw(
        200,
        "text/event-stream; charset=utf-8",
        body,
    )])
    .await;
    let client = client_for(&server);

    let mut stream = request()
        .with_session_id("continuation-session")
        .send_via(&client)
        .await
        .unwrap();

    let AgentStreamEvent::SessionCreated(created) = stream.next().await.unwrap().unwrap() else {
        panic!("expected session_created");
    };
    assert_eq!(created.session_id(), Some("server-session"));

    let AgentStreamEvent::Reasoning(reasoning) = stream.next().await.unwrap().unwrap() else {
        panic!("expected reasoning");
    };
    assert_eq!(reasoning.data(), Some("reasoning text"));

    let AgentStreamEvent::Thought(thought) = stream.next().await.unwrap().unwrap() else {
        panic!("expected thought");
    };
    assert_eq!(thought.data(), Some("thought text"));

    let AgentStreamEvent::ToolCall(call) = stream.next().await.unwrap().unwrap() else {
        panic!("expected tool_call");
    };
    assert_eq!(call.data().and_then(|data| data.call_id()), Some("call-1"));
    assert_eq!(
        call.data()
            .and_then(|data| data.arguments())
            .and_then(|arguments| arguments.get("q")),
        Some(&json!("x"))
    );

    let AgentStreamEvent::ToolResult(result) = stream.next().await.unwrap().unwrap() else {
        panic!("expected tool_result");
    };
    assert_eq!(
        result
            .data()
            .and_then(|data| data.status())
            .map(|status| status.as_str()),
        Some("future_status")
    );
    let status_debug = format!(
        "{:?}",
        result.data().and_then(|data| data.status()).unwrap()
    );
    assert!(!status_debug.contains("future_status"));
    assert_eq!(result.data().and_then(|data| data.duration_ms()), Some(4));

    let AgentStreamEvent::Answer(answer) = stream.next().await.unwrap().unwrap() else {
        panic!("expected answer");
    };
    assert_eq!(answer.data(), Some("answer text"));

    let AgentStreamEvent::Done(done) = stream.next().await.unwrap().unwrap() else {
        panic!("expected done");
    };
    assert_eq!(done.message_id(), Some("message-1"));
    assert_eq!(done.usage().and_then(|usage| usage.total_tokens()), Some(5));
    assert!(stream.next().await.is_none());

    let captured = server.requests();
    assert_eq!(captured.len(), 1);
    assert_eq!(captured[0].method, "POST");
    assert_eq!(captured[0].path, "/api/zrag/agent/chat");
    assert_eq!(
        captured[0].authorization.as_deref(),
        Some("Bearer test.12345678901234567890")
    );
    assert_eq!(
        request_header(&captured[0], "x-session-id"),
        Some("continuation-session")
    );
    assert_eq!(
        request_header(&captured[0], "accept"),
        Some("text/event-stream")
    );
    assert_eq!(
        request_header(&captured[0], "content-type"),
        Some("application/json")
    );

    let sent: Value = serde_json::from_slice(&captured[0].body).unwrap();
    assert!(sent.get("session_id").is_none());
    assert!(sent.get("X-Session-Id").is_none());
    assert_eq!(sent["messages"][0]["role"], "user");
    assert_eq!(
        sent["messages"][0]["content"][1]["image_url"]["url"],
        "https://example.test/image.png"
    );
    assert_eq!(sent["retrieval"]["know_ids"], json!(["knowledge-1"]));
}

#[tokio::test]
async fn zrag_sse_handshake_requires_an_unranged_http_200() {
    let terminal = "data: {\"type\":\"done\"}\n\n";
    let mut ranged_ok = ScriptedResponse::raw(200, "text/event-stream", terminal);
    ranged_ok
        .headers
        .push(("content-range".into(), "bytes 0-24/50".into()));

    for (case, response, expected_error) in [
        (
            "partial response with a valid terminal marker",
            ScriptedResponse::raw(206, "text/event-stream", terminal),
            Some("HTTP 200 OK"),
        ),
        (
            "no-content response",
            ScriptedResponse::raw(204, "text/event-stream", ""),
            Some("HTTP 200 OK"),
        ),
        (
            "ranged-looking 200 response",
            ranged_ok,
            Some("Content-Range"),
        ),
        (
            "complete 200 response",
            ScriptedResponse::raw(200, "text/event-stream", terminal),
            None,
        ),
    ] {
        let server = TestServer::start(vec![response]).await;
        let result = request().stream_via(&client_for(&server)).await;

        if let Some(expected_message) = expected_error {
            let error = match result {
                Ok(_) => panic!("{case} must not establish a ZRAG SSE stream"),
                Err(error) => error,
            };
            assert_eq!(error.code(), Some(codes::SDK_VALIDATION), "{case}");
            assert!(
                error.message().contains(expected_message),
                "{case}: {error}"
            );
            assert_eq!(
                error.request_metadata().map(|metadata| metadata.attempts()),
                Some(1),
                "{case}"
            );
        } else {
            let mut stream = result.expect("a complete 200 response must establish ZRAG SSE");
            assert!(stream.next().await.unwrap().unwrap().is_done());
            assert!(stream.next().await.is_none());
        }
        assert_eq!(server.requests().len(), 1, "{case}");
        server.shutdown().await;
    }
}

#[tokio::test]
async fn non_success_sse_status_preserves_business_projection_redaction_and_metadata() {
    let secret = "continuation-session";
    let mut response = ScriptedResponse::raw(
        429,
        "text/event-stream",
        json!({
            "error": {
                "code": 1302,
                "message": format!("session {secret} is rate limited")
            }
        })
        .to_string(),
    );
    response
        .headers
        .push(("x-request-id".into(), "req-sse-error".into()));
    response.headers.push(("retry-after".into(), "7".into()));
    let server = TestServer::start(vec![response]).await;

    let error = request()
        .with_session_id(secret)
        .stream_via(&client_for(&server))
        .await
        .unwrap_err();

    assert!(error.is_rate_limit());
    assert_eq!(error.code(), Some(1302));
    assert!(!error.to_string().contains(secret));
    let metadata = error.request_metadata().unwrap();
    assert_eq!(metadata.attempts(), 1);
    assert_eq!(metadata.request_id(), Some("req-sse-error"));
    assert_eq!(
        metadata.retry_after(),
        Some(std::time::Duration::from_secs(7))
    );
    assert_eq!(server.requests().len(), 1);
    server.shutdown().await;
}

#[tokio::test]
async fn invalid_request_and_session_header_fail_before_network_io() {
    let server = TestServer::start(vec![ScriptedResponse::raw(
        200,
        "text/event-stream",
        "data: {\"type\":\"done\"}\n\n",
    )])
    .await;
    let client = client_for(&server);

    let invalid = ZragChatRequest::new(
        vec![ZragChatMessage::user("question")],
        ZragChatRetrieval::new(vec!["knowledge-1".to_string()]),
    )
    .with_session_id("private session value");
    let error = invalid.stream_via(&client).await.unwrap_err();

    assert_eq!(error.code(), Some(codes::SDK_VALIDATION));
    assert!(!error.to_string().contains("private session value"));
    assert!(server.requests().is_empty());
}

#[tokio::test]
async fn unknown_type_is_preserved_without_debug_leakage() {
    let server = TestServer::start(vec![ScriptedResponse::raw(
        200,
        "text/event-stream",
        concat!(
            "data: {\"type\":\"future_private_type\",\"sessionId\":[\"future shape\"],\"payload\":\"private payload\"}\n\n",
            "data: {\"type\":\"done\"}\n\n",
        ),
    )])
    .await;
    let mut stream = request().stream_via(&client_for(&server)).await.unwrap();

    let AgentStreamEvent::Unknown(unknown) = stream.next().await.unwrap().unwrap() else {
        panic!("expected unknown event");
    };
    assert_eq!(unknown.event_type(), "future_private_type");
    assert_eq!(unknown.raw()["payload"], "private payload");
    let debug = format!("{unknown:?}");
    assert!(!debug.contains("future_private_type"));
    assert!(!debug.contains("private payload"));
    assert!(stream.next().await.unwrap().unwrap().is_done());
    assert!(stream.next().await.is_none());
}

#[tokio::test]
async fn type_error_yields_one_redacted_error_then_terminates() {
    let secret = "continuation-session";
    let body = format!(
        "data: {{\"type\":\"error\",\"data\":{{\"message\":\"session {secret} failed\"}}}}\n\ndata: {{\"type\":\"done\"}}\n\n"
    );
    let server =
        TestServer::start(vec![ScriptedResponse::raw(200, "text/event-stream", body)]).await;
    let mut stream = request()
        .with_session_id(secret)
        .stream_via(&client_for(&server))
        .await
        .unwrap();

    let error = stream.next().await.unwrap().unwrap_err();
    assert_eq!(error.code(), Some(codes::SDK_IO));
    assert!(!error.to_string().contains(secret));
    assert!(stream.next().await.is_none());
}

#[tokio::test]
async fn http_error_echoing_the_session_header_is_redacted() {
    let secret = "continuation-session";
    let server = TestServer::start(vec![ScriptedResponse::json(
        400,
        json!({
            "error": {
                "code": 1200,
                "message": format!("invalid session {secret}")
            }
        }),
    )])
    .await;

    let error = request()
        .with_session_id(secret)
        .stream_via(&client_for(&server))
        .await
        .unwrap_err();
    assert_eq!(error.code(), Some(1200));
    assert!(!error.to_string().contains(secret));
    assert_eq!(server.requests().len(), 1);
}

#[tokio::test]
async fn http_error_request_ids_containing_the_session_are_omitted() {
    let secret = "continuation-session";
    let envelope_server = TestServer::start(vec![ScriptedResponse::json(
        400,
        json!({
            "error": {"code": 1200, "message": "invalid session"},
            "request_id": secret
        }),
    )])
    .await;
    let envelope_error = request()
        .with_session_id(secret)
        .stream_via(&client_for(&envelope_server))
        .await
        .unwrap_err();
    assert_eq!(
        envelope_error
            .request_metadata()
            .and_then(|metadata| metadata.request_id()),
        None
    );

    let mut header_response = ScriptedResponse::json(
        400,
        json!({"error": {"code": 1200, "message": "invalid session"}}),
    );
    header_response
        .headers
        .push(("x-request-id".to_string(), format!("x-{secret}-y")));
    let header_server = TestServer::start(vec![header_response]).await;
    let header_error = request()
        .with_session_id(secret)
        .stream_via(&client_for(&header_server))
        .await
        .unwrap_err();
    assert_eq!(
        header_error
            .request_metadata()
            .and_then(|metadata| metadata.request_id()),
        None
    );
}

fn assert_error_omits_numeric_session(error: &zai_rs::ZaiError, secret: &str) {
    assert!(!error.to_string().contains(secret));
    assert!(!format!("{error:?}").contains(secret));
    assert!(!error.message().contains(secret));
    assert_ne!(error.code(), secret.parse::<u16>().ok());
    assert!(
        error
            .raw_business_code()
            .is_none_or(|code| !code.contains(secret))
    );
    assert!(
        error
            .request_metadata()
            .and_then(|metadata| metadata.request_id())
            .is_none_or(|request_id| !request_id.contains(secret))
    );
    assert_eq!(
        error
            .request_metadata()
            .and_then(|metadata| metadata.retry_after()),
        None
    );
}

#[tokio::test]
async fn http_error_business_codes_echoing_a_numeric_session_are_omitted() {
    let secret = "1200";
    let mut recognized_response = ScriptedResponse::json(
        400,
        json!({"error": {"code": 1200, "message": "invalid session"}}),
    );
    recognized_response
        .headers
        .push(("retry-after".to_string(), secret.to_string()));
    let recognized_server = TestServer::start(vec![recognized_response]).await;
    let recognized_error = request()
        .with_session_id(secret)
        .stream_via(&client_for(&recognized_server))
        .await
        .unwrap_err();
    assert_error_omits_numeric_session(&recognized_error, secret);

    let mut raw_response = ScriptedResponse::raw(
        400,
        "application/json",
        r#"{"code":1200,"message":"invalid session"#,
    );
    raw_response
        .headers
        .push(("retry-after".to_string(), secret.to_string()));
    let raw_server = TestServer::start(vec![raw_response]).await;
    let raw_error = request()
        .with_session_id(secret)
        .stream_via(&client_for(&raw_server))
        .await
        .unwrap_err();
    assert_error_omits_numeric_session(&raw_error, secret);
}

#[tokio::test]
async fn eof_before_done_yields_one_error_then_terminates() {
    let server = TestServer::start(vec![ScriptedResponse::raw(
        200,
        "text/event-stream",
        "data: {\"type\":\"answer\",\"data\":\"partial\"}\n\n",
    )])
    .await;
    let mut stream = request().stream_via(&client_for(&server)).await.unwrap();

    assert!(matches!(
        stream.next().await.unwrap().unwrap(),
        AgentStreamEvent::Answer(_)
    ));
    let error = stream.next().await.unwrap().unwrap_err();
    assert_eq!(error.code(), Some(codes::SDK_IO));
    assert!(error.message().contains("type=done"));
    assert!(stream.next().await.is_none());
}

#[tokio::test]
async fn literal_done_is_not_a_zrag_terminator() {
    let server = TestServer::start(vec![ScriptedResponse::raw(
        200,
        "text/event-stream",
        "data: [DONE]\n\n",
    )])
    .await;
    let mut stream = request().stream_via(&client_for(&server)).await.unwrap();

    let error = stream.next().await.unwrap().unwrap_err();
    assert_eq!(error.code(), Some(codes::SDK_VALIDATION));
    assert!(error.message().contains("literal [DONE] is invalid"));
    assert!(stream.next().await.is_none());
}

#[tokio::test]
async fn streaming_post_is_not_replayed_after_http_failure() {
    let server = TestServer::start(vec![
        ScriptedResponse::json(503, json!({"error": {"code": 1234, "message": "busy"}})),
        ScriptedResponse::raw(200, "text/event-stream", "data: {\"type\":\"done\"}\n\n"),
    ])
    .await;

    let error = request()
        .stream_via(&client_for(&server))
        .await
        .unwrap_err();
    assert_eq!(
        error.request_metadata().map(|metadata| metadata.attempts()),
        Some(1)
    );
    assert_eq!(server.requests().len(), 1);
}
