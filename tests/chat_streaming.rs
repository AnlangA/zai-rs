//! End-to-end tests for the authenticated typed chat SSE path.

mod support;

use std::{sync::Arc, time::Duration};

use serde_json::json;
use support::http_server::{ScriptedResponse, TestServer};
use zai_rs::client::{ApiFamily, ZaiClient};
use zai_rs::model::{ChatCompletion, GLM5_2, TextMessage};

const TEST_KEY: &str = "test.12345678901234567890";

fn client_for(server: &TestServer) -> ZaiClient {
    ZaiClient::builder(TEST_KEY)
        .allow_insecure_transport(true)
        .endpoint(
            ApiFamily::PaasV4,
            format!("{}/api/paas/v4", server.base_url),
        )
        .build()
        .unwrap()
}

#[tokio::test]
async fn stream_via_keeps_auth_internal_and_decodes_events() {
    let first = json!({
        "id": "chat-1",
        "choices": [{"index": 0, "delta": {"content": "你"}}]
    });
    let second = json!({
        "id": "chat-1",
        "choices": [{"index": 0, "delta": {"content": "好"}}]
    });
    let body = format!("data: {first}\n\ndata: {second}\n\ndata: [DONE]\n\n");
    let server = TestServer::start(vec![ScriptedResponse::raw(
        200,
        "text/event-stream; charset=utf-8",
        body,
    )])
    .await;
    let client = client_for(&server);

    let mut stream = ChatCompletion::new(GLM5_2 {}, TextMessage::user("hello"))
        .enable_stream()
        .stream_via(&client)
        .await
        .unwrap();
    let first = stream.next().await.unwrap().unwrap();
    let second = stream.next().await.unwrap().unwrap();
    assert!(stream.next().await.is_none());
    assert_eq!(
        first.choices[0].delta.as_ref().unwrap().content.as_deref(),
        Some("你")
    );
    assert_eq!(
        second.choices[0].delta.as_ref().unwrap().content.as_deref(),
        Some("好")
    );

    let requests = server.requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].method, "POST");
    assert_eq!(requests[0].path, "/api/paas/v4/chat/completions");
    let expected_authorization = format!("Bearer {TEST_KEY}");
    assert_eq!(
        requests[0].authorization.as_deref(),
        Some(expected_authorization.as_str())
    );
    assert!(
        requests[0]
            .headers
            .iter()
            .any(|(name, value)| name == "accept" && value == "text/event-stream")
    );
    let request_body: serde_json::Value = serde_json::from_slice(&requests[0].body).unwrap();
    assert_eq!(request_body["stream"], true);
    server.shutdown().await;
}

#[tokio::test]
async fn chat_sse_handshake_requires_an_unranged_http_200() {
    let terminal = "data: [DONE]\n\n";
    let mut ranged_ok = ScriptedResponse::raw(200, "text/event-stream", terminal);
    ranged_ok
        .headers
        .push(("content-range".into(), "bytes 0-13/28".into()));

    for (case, response, expected_error) in [
        (
            "partial response with a valid terminal marker",
            ScriptedResponse::raw(206, "text/event-stream", terminal),
            Some("HTTP 200 OK"),
        ),
        (
            "created response",
            ScriptedResponse::raw(201, "text/event-stream", terminal),
            Some("HTTP 200 OK"),
        ),
        (
            "accepted response",
            ScriptedResponse::raw(202, "text/event-stream", terminal),
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
        let result = ChatCompletion::new(GLM5_2 {}, TextMessage::user("hello"))
            .enable_stream()
            .stream_via(&client_for(&server))
            .await;

        if let Some(expected_message) = expected_error {
            let error = match result {
                Ok(_) => panic!("{case} must not establish a chat SSE stream"),
                Err(error) => error,
            };
            assert_eq!(
                error.code(),
                Some(zai_rs::client::error::codes::SDK_VALIDATION),
                "{case}"
            );
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
            let mut stream = result.expect("a complete 200 response must establish chat SSE");
            assert!(stream.next().await.is_none());
        }
        assert_eq!(server.requests().len(), 1, "{case}");
        server.shutdown().await;
    }
}

#[tokio::test]
async fn invalid_success_status_or_range_fails_before_polling_sse_body() {
    let terminal = bytes::Bytes::from_static(b"data: [DONE]\n\n");

    for (case, status, content_range) in [
        ("partial response", 206, None),
        ("created response", 201, None),
        ("ranged-looking 200 response", 200, Some("bytes 0-13/28")),
    ] {
        let gate = Arc::new(tokio::sync::Semaphore::new(0));
        let mut response =
            ScriptedResponse::chunked(status, "text/event-stream", [terminal.clone()])
                .with_chunk_gate(Arc::clone(&gate));
        if let Some(value) = content_range {
            response
                .headers
                .push(("content-range".into(), value.into()));
        }
        let server = TestServer::start(vec![response]).await;

        let result = tokio::time::timeout(
            Duration::from_millis(500),
            ChatCompletion::new(GLM5_2 {}, TextMessage::user("hello"))
                .enable_stream()
                .stream_via(&client_for(&server)),
        )
        .await
        .unwrap_or_else(|_| panic!("{case} polled the gated response body"));
        let error = match result {
            Ok(_) => panic!("{case} unexpectedly established an SSE stream"),
            Err(error) => error,
        };
        assert_eq!(
            error.code(),
            Some(zai_rs::client::error::codes::SDK_VALIDATION),
            "{case}"
        );
        assert_eq!(gate.available_permits(), 0, "{case}");
        assert_eq!(server.requests().len(), 1, "{case}");
        server.shutdown().await;
    }
}

#[tokio::test]
async fn stream_via_ignores_future_tool_call_types_and_continues_to_done() {
    let future_tool = json!({
        "id": "chat-future-tool",
        "choices": [{
            "index": 0,
            "delta": {
                "tool_calls": [{
                    "index": 2,
                    "id": "call-future",
                    "type": "computer",
                    "function": {"name": "lookup", "arguments": "{\"city\":"}
                }]
            }
        }]
    });
    let text = json!({
        "id": "chat-future-tool",
        "choices": [{"index": 0, "delta": {"content": "still streaming"}}]
    });
    let body = format!("data: {future_tool}\n\ndata: {text}\n\ndata: [DONE]\n\n");
    let server =
        TestServer::start(vec![ScriptedResponse::raw(200, "text/event-stream", body)]).await;
    let client = client_for(&server);

    let mut stream = ChatCompletion::new(GLM5_2 {}, TextMessage::user("hello"))
        .enable_stream()
        .stream_via(&client)
        .await
        .unwrap();

    let first = stream.next().await.unwrap().unwrap();
    let calls = first.choices[0]
        .delta
        .as_ref()
        .unwrap()
        .tool_calls
        .as_ref()
        .unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].index, Some(2));
    assert_eq!(calls[0].id.as_deref(), Some("call-future"));
    assert!(calls[0].type_.is_none());
    let function = calls[0].function.as_ref().unwrap();
    assert_eq!(function.name.as_deref(), Some("lookup"));
    assert_eq!(function.arguments.as_deref(), Some("{\"city\":"));

    let second = stream.next().await.unwrap().unwrap();
    assert_eq!(
        second.choices[0].delta.as_ref().unwrap().content.as_deref(),
        Some("still streaming")
    );
    assert!(stream.next().await.is_none());
    server.shutdown().await;
}

#[tokio::test]
async fn stream_via_validates_builders_added_after_enable_stream() {
    let server = TestServer::start(vec![ScriptedResponse::raw(
        200,
        "text/event-stream",
        "data: [DONE]\n\n",
    )])
    .await;
    let client = client_for(&server);

    let result = ChatCompletion::new(GLM5_2 {}, TextMessage::user("hello"))
        .enable_stream()
        .with_temperature(2.0)
        .add_message(TextMessage::assistant("still local"))
        .stream_via(&client)
        .await;
    let error = match result {
        Ok(_) => panic!("invalid streaming request must fail before transport"),
        Err(error) => error,
    };

    assert_eq!(
        error.code(),
        Some(zai_rs::client::error::codes::SDK_VALIDATION)
    );
    assert!(
        server.requests().is_empty(),
        "validation failure must not reach the transport"
    );
    server.shutdown().await;
}

#[tokio::test]
async fn stream_via_preserves_business_errors_before_streaming() {
    let server = TestServer::start(vec![ScriptedResponse::json(
        429,
        json!({"error": {"code": 1302, "message": "rate limited"}}),
    )])
    .await;
    let client = client_for(&server);

    let error = match ChatCompletion::new(GLM5_2 {}, TextMessage::user("hello"))
        .enable_stream()
        .stream_via(&client)
        .await
    {
        Ok(_) => panic!("expected the SSE handshake to fail"),
        Err(error) => error,
    };
    assert!(error.is_rate_limit());
    assert_eq!(error.code(), Some(1302));
    assert_eq!(server.requests().len(), 1, "streaming POST must not retry");
    server.shutdown().await;
}

#[tokio::test]
async fn sse_handshake_business_duplicates_fail_closed_without_body_leakage() {
    let server = TestServer::start(vec![ScriptedResponse::raw(
        429,
        "application/json",
        r#"{"error":{"code":1302},"error":{"code":200},"message":"private-sse-payload"}"#,
    )])
    .await;
    let client = client_for(&server);

    let error = match ChatCompletion::new(GLM5_2 {}, TextMessage::user("hello"))
        .enable_stream()
        .stream_via(&client)
        .await
    {
        Ok(_) => panic!("ambiguous SSE handshake became a stream"),
        Err(error) => error,
    };

    assert!(error.is_rate_limit());
    assert_eq!(error.code(), Some(429));
    assert!(
        error
            .message()
            .contains("ambiguous JSON business-error envelope")
    );
    for rendered in [error.to_string(), format!("{error:?}"), error.compact()] {
        assert!(!rendered.contains("private-sse-payload"));
        assert!(!rendered.contains("1302"));
    }
    assert_eq!(server.requests().len(), 1, "streaming POST must not retry");
    server.shutdown().await;
}

#[tokio::test]
async fn stream_via_rejects_eof_without_done() {
    let chunk = json!({
        "id": "chat-1",
        "choices": [{"index": 0, "delta": {"content": "partial"}}]
    });
    let server = TestServer::start(vec![ScriptedResponse::raw(
        200,
        "text/event-stream",
        format!("data: {chunk}\n\n"),
    )])
    .await;
    let client = client_for(&server);
    let mut stream = ChatCompletion::new(GLM5_2 {}, TextMessage::user("hello"))
        .enable_stream()
        .stream_via(&client)
        .await
        .unwrap();

    assert!(stream.next().await.unwrap().is_ok());
    let error = stream.next().await.unwrap().unwrap_err();
    assert_eq!(error.code(), Some(zai_rs::client::error::codes::SDK_IO));
    assert!(error.message().contains("[DONE]"));
    assert!(stream.next().await.is_none());
    server.shutdown().await;
}

#[tokio::test]
async fn stream_via_maps_in_band_business_errors() {
    let body = concat!(
        "data: {\"error\":{\"code\":1302,\"message\":\"rate limited\"}}\n\n",
        "data: [DONE]\n\n"
    );
    let server =
        TestServer::start(vec![ScriptedResponse::raw(200, "text/event-stream", body)]).await;
    let client = client_for(&server);
    let mut stream = ChatCompletion::new(GLM5_2 {}, TextMessage::user("hello"))
        .enable_stream()
        .stream_via(&client)
        .await
        .unwrap();

    let error = stream.next().await.unwrap().unwrap_err();
    assert!(error.is_rate_limit());
    assert_eq!(error.code(), Some(1302));
    assert!(stream.next().await.is_none());
    server.shutdown().await;
}

#[tokio::test]
async fn in_band_reserved_duplicates_fail_closed_once_without_payload_leakage() {
    for (case, payload, secret) in [
        (
            "top-level code",
            r#"{"id":"chat-duplicate-code","choices":[{"index":0,"delta":{"content":"valid"}}],"code":1302,"code":200,"message":"private-chat-top-level"}"#,
            "private-chat-top-level",
        ),
        (
            "nested error code",
            r#"{"id":"chat-duplicate-nested-code","choices":[{"index":0,"delta":{"content":"valid"}}],"error":{"code":1302,"code":200,"message":"private-chat-nested"}}"#,
            "private-chat-nested",
        ),
    ] {
        // Apart from the ignored envelope-shaped extension, this is a valid
        // typed chat chunk. The transport probe must reject the ambiguity
        // before serde can turn it into a successful stream item.
        assert!(serde_json::from_str::<zai_rs::model::ChatStreamResponse>(payload).is_ok());

        let body = format!(
            "data: {payload}\n\ndata: {{\"id\":\"after-error\",\"choices\":[]}}\n\ndata: [DONE]\n\n"
        );
        let server =
            TestServer::start(vec![ScriptedResponse::raw(200, "text/event-stream", body)]).await;
        let mut stream = ChatCompletion::new(GLM5_2 {}, TextMessage::user("hello"))
            .enable_stream()
            .stream_via(&client_for(&server))
            .await
            .unwrap();

        let error = stream.next().await.unwrap().unwrap_err();
        assert_eq!(
            error.code(),
            Some(zai_rs::client::error::codes::SDK_VALIDATION),
            "{case}"
        );
        assert_eq!(
            error.message(),
            "ambiguous JSON business-error envelope (duplicate reserved field)",
            "{case}"
        );
        for rendered in [error.to_string(), format!("{error:?}"), error.compact()] {
            assert!(!rendered.contains(secret), "{case}: {rendered}");
            assert!(!rendered.contains("1302"), "{case}: {rendered}");
        }
        assert!(stream.next().await.is_none(), "{case}");
        server.shutdown().await;
    }
}

#[tokio::test]
async fn stream_via_maps_error_finish_reasons_once_then_terminates() {
    for (reason, expected_code, retryable) in [
        ("sensitive", 1301, false),
        ("network_error", 1234, true),
        ("model_context_window_exceeded", 1261, false),
    ] {
        let body = format!(
            "data: {{\"choices\":[{{\"index\":0,\"delta\":{{}},\"finish_reason\":\"{reason}\"}}]}}\n\ndata: [DONE]\n\n"
        );
        let server =
            TestServer::start(vec![ScriptedResponse::raw(200, "text/event-stream", body)]).await;
        let client = client_for(&server);
        let mut stream = ChatCompletion::new(GLM5_2 {}, TextMessage::user("hello"))
            .enable_stream()
            .stream_via(&client)
            .await
            .unwrap();

        let error = stream.next().await.unwrap().unwrap_err();
        assert_eq!(error.code(), Some(expected_code));
        assert_eq!(error.is_retryable(), retryable);
        assert!(stream.next().await.is_none());
        server.shutdown().await;
    }
}
