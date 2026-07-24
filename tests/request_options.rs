//! End-to-end contracts for scoped HTTP request policy overrides.

mod support;

use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use serde_json::json;
use support::http_server::{ScriptedResponse, TestServer};
use zai_rs::client::{
    ApiFamily, HttpTransportConfig, RequestOptions, RetryOverride, TimeoutPhase, ZaiClient,
    ZaiError,
};
use zai_rs::file::{FileListPurpose, FileListRequest};
use zai_rs::model::{ChatCompletion, GLM5_2, TextMessage};

const TEST_KEY: &str = "test.12345678901234567890";

fn client_for(server: &TestServer, transport: HttpTransportConfig) -> ZaiClient {
    ZaiClient::builder(TEST_KEY)
        .allow_insecure_transport(true)
        .endpoint(
            ApiFamily::PaasV4,
            format!("{}/api/paas/v4", server.base_url),
        )
        .transport(transport)
        .build()
        .unwrap()
}

fn empty_file_list() -> serde_json::Value {
    json!({"object": "list", "data": [], "has_more": false})
}

#[tokio::test]
async fn scoped_attempt_timeout_does_not_mutate_the_shared_client() {
    let server = TestServer::start(vec![
        ScriptedResponse::json(200, empty_file_list()).with_delay(Duration::from_millis(100)),
        ScriptedResponse::json(200, empty_file_list()),
    ])
    .await;
    let transport = HttpTransportConfig::default()
        .with_request_timeout(Duration::from_secs(1))
        .unwrap()
        .with_max_attempts(1)
        .unwrap();
    let client = client_for(&server, transport);
    let scoped = client.clone().with_request_options(
        RequestOptions::default()
            .with_attempt_timeout(Duration::from_millis(20))
            .unwrap()
            .with_overall_timeout(Duration::from_secs(1))
            .unwrap(),
    );

    let error = FileListRequest::new(FileListPurpose::Batch)
        .send_via(&scoped)
        .await
        .unwrap_err();
    assert_eq!(
        error.code(),
        Some(zai_rs::client::error::codes::SDK_TIMEOUT)
    );
    let metadata = error.request_metadata().unwrap();
    assert_eq!(metadata.attempts(), 1);
    assert_eq!(metadata.timeout_phase(), Some(TimeoutPhase::Attempt));
    assert!(client.request_options().attempt_timeout().is_none());

    let response = FileListRequest::new(FileListPurpose::Batch)
        .send_via(&client)
        .await
        .unwrap();
    assert_eq!(response.has_more, Some(false));
    assert_eq!(server.requests().len(), 2);
    server.shutdown().await;
}

#[tokio::test]
async fn non_idempotent_retry_requires_the_scoped_assertion() {
    let success = json!({
        "id": "chat-retry",
        "model": "glm-5.2",
        "choices": [{
            "index": 0,
            "message": {"role": "assistant", "content": "ok"},
            "finish_reason": "stop"
        }],
        "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2}
    });
    let server = TestServer::start(vec![
        ScriptedResponse::json(500, json!({"message": "temporary"})),
        ScriptedResponse::json(200, success),
    ])
    .await;
    let transport = HttpTransportConfig::default().with_max_attempts(2).unwrap();
    let client = client_for(&server, transport).with_request_options(
        RequestOptions::default()
            .with_max_attempts(2)
            .unwrap()
            .with_retry_override(RetryOverride::AssumeIdempotent),
    );

    let response = ChatCompletion::new(GLM5_2 {}, TextMessage::user("hello"))
        .send_via(&client)
        .await
        .unwrap();
    assert_eq!(response.id.as_deref(), Some("chat-retry"));
    assert_eq!(server.requests().len(), 2);
    server.shutdown().await;
}

#[tokio::test]
async fn scoped_attempt_count_cannot_raise_the_global_cap() {
    let server = TestServer::start(vec![
        ScriptedResponse::json(500, json!({"message": "temporary"})),
        ScriptedResponse::json(200, empty_file_list()),
    ])
    .await;
    let transport = HttpTransportConfig::default().with_max_attempts(1).unwrap();
    let client = client_for(&server, transport).with_request_options(
        RequestOptions::default()
            .with_max_attempts(3)
            .unwrap()
            .with_retry_override(RetryOverride::AssumeIdempotent),
    );

    FileListRequest::new(FileListPurpose::Batch)
        .send_via(&client)
        .await
        .unwrap_err();
    assert_eq!(server.requests().len(), 1);
    server.shutdown().await;
}

#[tokio::test]
async fn overall_deadline_includes_retry_after_backoff() {
    let mut busy = ScriptedResponse::json(503, json!({"message": "busy"}));
    busy.headers.push(("retry-after".into(), "1".into()));
    let server =
        TestServer::start(vec![busy, ScriptedResponse::json(200, empty_file_list())]).await;
    let client = client_for(&server, HttpTransportConfig::default()).with_request_options(
        RequestOptions::default()
            .with_overall_timeout(Duration::from_millis(50))
            .unwrap(),
    );

    let error = FileListRequest::new(FileListPurpose::Batch)
        .send_via(&client)
        .await
        .unwrap_err();
    assert_eq!(
        error.code(),
        Some(zai_rs::client::error::codes::SDK_TIMEOUT)
    );
    assert!(error.message().contains("overall"));
    let metadata = error.request_metadata().unwrap();
    assert_eq!(metadata.attempts(), 1);
    assert_eq!(metadata.timeout_phase(), Some(TimeoutPhase::Overall));
    assert_eq!(metadata.retry_after(), Some(Duration::from_secs(1)));
    assert_eq!(server.requests().len(), 1);
    server.shutdown().await;
}

#[tokio::test]
async fn retry_override_never_replays_an_sse_post() {
    let event = json!({
        "id": "unexpected-retry",
        "choices": [{"index": 0, "delta": {"content": "unexpected"}}]
    });
    let server = TestServer::start(vec![
        ScriptedResponse::json(503, json!({"message": "busy"})),
        ScriptedResponse::raw(
            200,
            "text/event-stream",
            format!("data: {event}\n\ndata: [DONE]\n\n"),
        ),
    ])
    .await;
    let client = client_for(&server, HttpTransportConfig::default()).with_request_options(
        RequestOptions::default()
            .with_max_attempts(3)
            .unwrap()
            .with_retry_override(RetryOverride::AssumeIdempotent),
    );

    let error = match ChatCompletion::new(GLM5_2 {}, TextMessage::user("hello"))
        .enable_stream()
        .stream_via(&client)
        .await
    {
        Ok(_) => panic!("streaming POST unexpectedly retried"),
        Err(error) => error,
    };
    assert!(error.is_server_error());
    assert_eq!(server.requests().len(), 1);
    server.shutdown().await;
}

#[tokio::test]
async fn sse_handshake_and_idle_deadlines_are_independent() {
    let handshake_server = TestServer::start(vec![
        ScriptedResponse::raw(200, "text/event-stream", "data: [DONE]\n\n")
            .with_delay(Duration::from_millis(100)),
    ])
    .await;
    let handshake_client = client_for(&handshake_server, HttpTransportConfig::default())
        .with_request_options(
            RequestOptions::default()
                .with_sse_handshake_timeout(Duration::from_millis(20))
                .unwrap(),
        );
    let handshake_error = match ChatCompletion::new(GLM5_2 {}, TextMessage::user("hello"))
        .enable_stream()
        .stream_via(&handshake_client)
        .await
    {
        Ok(_) => panic!("expected the SSE handshake to time out"),
        Err(error) => error,
    };
    assert_eq!(
        handshake_error.code(),
        Some(zai_rs::client::error::codes::SDK_TIMEOUT)
    );
    assert!(handshake_error.message().contains("handshake"));
    let metadata = handshake_error.request_metadata().unwrap();
    assert_eq!(metadata.attempts(), 1);
    assert_eq!(metadata.timeout_phase(), Some(TimeoutPhase::SseHandshake));
    handshake_server.shutdown().await;

    let first = json!({
        "id": "chat-idle",
        "choices": [{"index": 0, "delta": {"content": "first"}}]
    });
    let gate = Arc::new(tokio::sync::Semaphore::new(1));
    let idle_server = TestServer::start(vec![
        ScriptedResponse::chunked(
            200,
            "text/event-stream",
            [
                Bytes::from(format!("data: {first}\n\n")),
                Bytes::from_static(b"data: [DONE]\n\n"),
            ],
        )
        .with_chunk_gate(Arc::clone(&gate)),
    ])
    .await;
    let idle_client = client_for(&idle_server, HttpTransportConfig::default())
        .with_request_options(
            RequestOptions::default()
                .with_sse_idle_timeout(Duration::from_millis(20))
                .unwrap(),
        );
    let mut stream = ChatCompletion::new(GLM5_2 {}, TextMessage::user("hello"))
        .enable_stream()
        .stream_via(&idle_client)
        .await
        .unwrap();
    assert!(stream.next().await.unwrap().is_ok());
    // The idle deadline is absolute from the last delivered transport chunk;
    // pausing the consumer must not reset it on the next poll.
    tokio::time::sleep(Duration::from_millis(30)).await;
    let idle_error = stream.next().await.unwrap().unwrap_err();
    gate.add_permits(1);
    assert_eq!(
        idle_error.code(),
        Some(zai_rs::client::error::codes::SDK_TIMEOUT)
    );
    assert!(idle_error.message().contains("idle"));
    let metadata = idle_error.request_metadata().unwrap();
    assert_eq!(metadata.attempts(), 1);
    assert_eq!(metadata.timeout_phase(), Some(TimeoutPhase::SseIdle));
    assert!(stream.next().await.is_none());
    idle_server.shutdown().await;
}

#[tokio::test]
async fn final_http_error_exposes_safe_structured_diagnostics() {
    let mut limited = ScriptedResponse::json(
        429,
        json!({
            "code": 1302,
            "message": "slow down",
            "request_id": "provider-request-42"
        }),
    );
    limited.headers.push(("retry-after".into(), "2".into()));
    let server = TestServer::start(vec![
        ScriptedResponse::json(503, json!({"message": "temporary"})),
        limited,
    ])
    .await;
    let transport = HttpTransportConfig::default().with_max_attempts(2).unwrap();
    let client = client_for(&server, transport);

    let error = FileListRequest::new(FileListPurpose::Batch)
        .send_via(&client)
        .await
        .unwrap_err();

    assert!(matches!(
        error.source_error(),
        ZaiError::RateLimitError { code: 1302, .. }
    ));
    let metadata = error.request_metadata().unwrap();
    assert_eq!(metadata.request_id(), Some("provider-request-42"));
    assert_eq!(metadata.attempts(), 2);
    assert_eq!(metadata.timeout_phase(), None);
    assert_eq!(metadata.retry_after(), Some(Duration::from_secs(2)));
    for rendered in [error.to_string(), format!("{error:?}"), error.compact()] {
        assert!(!rendered.contains("provider-request-42"));
    }
    assert_eq!(server.requests().len(), 2);
    server.shutdown().await;
}
