//! End-to-end tests for the authenticated typed chat SSE path.

mod support;

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
