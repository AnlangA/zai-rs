//! Unknown business codes must not hide actionable HTTP recovery semantics.

mod support;

use std::{sync::Arc, time::Duration};

use bytes::Bytes;
use serde_json::{Value, json};
use support::http_server::{ScriptedResponse, TestServer};
use zai_rs::model::{ChatCompletion, GLM5_2, TextMessage};
use zai_rs::{
    client::{ApiFamily, ErrorCategory, HttpTransportConfig, ZaiClient, error::codes},
    file::{FileListPurpose, FileListRequest},
    model::text_to_audio::{GlmTts, TextToAudioRequest},
};

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

fn single_attempt_client_for(server: &TestServer) -> ZaiClient {
    ZaiClient::builder(TEST_KEY)
        .allow_insecure_transport(true)
        .endpoint(
            ApiFamily::PaasV4,
            format!("{}/api/paas/v4", server.base_url),
        )
        .transport(HttpTransportConfig::default().with_max_attempts(1).unwrap())
        .build()
        .unwrap()
}

#[tokio::test]
async fn unknown_number_text_and_large_codes_fall_back_to_http_semantics() {
    let code_cases: [(Value, &str); 3] = [
        (json!(7777), "7777"),
        (json!("UPSTREAM_BUSY"), r#""UPSTREAM_BUSY""#),
        (json!(70_000), "70000"),
    ];
    let status_cases = [
        (401, ErrorCategory::Auth, false),
        (429, ErrorCategory::RateLimit, true),
        (503, ErrorCategory::Server, true),
    ];

    for (status, category, retryable) in status_cases {
        for (wire_code, diagnostic) in &code_cases {
            let server = TestServer::start(vec![ScriptedResponse::json(
                status,
                json!({
                    "error": {
                        "code": wire_code,
                        "message": "upstream rejected request"
                    }
                }),
            )])
            .await;
            let client = client_for(&server);

            let error = ChatCompletion::new(GLM5_2 {}, TextMessage::user("hello"))
                .send_via(&client)
                .await
                .expect_err("the business error envelope must not decode as success");

            assert_eq!(error.code(), Some(status));
            assert_eq!(error.raw_business_code(), Some(*diagnostic));
            assert_eq!(error.category(), category);
            assert_eq!(error.is_retryable(), retryable);
            assert!(
                !error.to_string().contains(diagnostic),
                "Display must not emit the diagnostic business code"
            );
            assert!(
                !format!("{error:?}").contains(diagnostic),
                "Debug must not emit the diagnostic business code"
            );
            server.shutdown().await;
        }
    }
}

#[tokio::test]
async fn flat_success_business_codes_cannot_hide_non_success_http_status() {
    for (status, wire_code, category) in [
        (400, json!(200), ErrorCategory::Client),
        (503, json!("200"), ErrorCategory::Server),
    ] {
        let server = TestServer::start(vec![ScriptedResponse::json(
            status,
            json!({"code": wire_code, "message": "HTTP request failed"}),
        )])
        .await;

        let error = FileListRequest::new(FileListPurpose::Batch)
            .send_via(&single_attempt_client_for(&server))
            .await
            .expect_err("a flat success code must not override non-success HTTP status");

        assert_eq!(error.code(), Some(status));
        assert_eq!(error.raw_business_code(), None);
        assert_eq!(error.category(), category);
        assert_eq!(server.requests().len(), 1);
        server.shutdown().await;
    }
}

#[tokio::test]
async fn ambiguous_business_envelope_preserves_only_http_classification() {
    for (status, category, retryable) in [
        (401, ErrorCategory::Auth, false),
        (429, ErrorCategory::RateLimit, true),
        (503, ErrorCategory::Server, true),
    ] {
        let server = TestServer::start(vec![ScriptedResponse::raw(
            status,
            "application/json",
            r#"{"code":1113,"code":1302,"message":"private-ambiguous-payload"}"#,
        )])
        .await;

        let error = FileListRequest::new(FileListPurpose::Batch)
            .send_via(&single_attempt_client_for(&server))
            .await
            .expect_err("ambiguous envelope escaped HTTP error handling");

        assert_eq!(error.code(), Some(status));
        assert_eq!(error.category(), category);
        assert_eq!(error.is_retryable(), retryable);
        assert_eq!(error.raw_business_code(), None);
        assert!(
            error
                .message()
                .contains("ambiguous JSON business-error envelope")
        );
        for rendered in [error.to_string(), format!("{error:?}"), error.compact()] {
            assert!(!rendered.contains("private-ambiguous-payload"));
            assert!(!rendered.contains("1113"));
            assert!(!rendered.contains("1302"));
        }
        assert_eq!(server.requests().len(), 1);
        server.shutdown().await;
    }
}

#[tokio::test]
async fn ambiguous_business_envelope_retries_from_http_status_without_stale_cache() {
    let server = TestServer::start(vec![
        ScriptedResponse::raw(
            503,
            "application/json",
            r#"{"code":1113,"code":1302,"message":"private-first-attempt"}"#,
        ),
        ScriptedResponse::json(200, json!({"object": "list", "data": []})),
    ])
    .await;

    let response = FileListRequest::new(FileListPurpose::Batch)
        .send_via(&client_for(&server))
        .await
        .expect("HTTP 503 must drive retry and the final clean probe must win");

    assert!(response.data.as_deref().is_some_and(|data| data.is_empty()));
    assert_eq!(server.requests().len(), 2);
    server.shutdown().await;
}

#[tokio::test]
async fn oversized_error_body_cannot_hide_retryable_http_status() {
    let server = TestServer::start(vec![
        ScriptedResponse::raw(503, "text/html", Bytes::from(vec![b'x'; 64 * 1024 + 1])),
        ScriptedResponse::json(200, json!({"object": "list", "data": []})),
    ])
    .await;

    let response = FileListRequest::new(FileListPurpose::Batch)
        .send_via(&client_for(&server))
        .await
        .expect("the idempotent GET must retry the oversized 503 response");

    assert_eq!(response.data.map_or(0, |items| items.len()), 0);
    assert_eq!(
        server.requests().len(),
        2,
        "the 503 status must remain visible to retry classification"
    );
    server.shutdown().await;
}

#[tokio::test]
async fn oversized_error_body_preserves_final_http_category() {
    let server = TestServer::start(vec![ScriptedResponse::raw(
        401,
        "text/html",
        Bytes::from(vec![b'x'; 64 * 1024 + 1]),
    )])
    .await;

    let error = FileListRequest::new(FileListPurpose::Batch)
        .send_via(&client_for(&server))
        .await
        .expect_err("the request must retain its HTTP authentication failure");

    assert_eq!(error.code(), Some(401));
    assert_eq!(error.category(), ErrorCategory::Auth);
    assert!(!error.is_retryable());
    assert_eq!(server.requests().len(), 1);
    server.shutdown().await;
}

#[tokio::test]
async fn truncated_json_uses_http_status_only_for_retry() {
    let mut body = br#"{"request_id":"request-1","code":1113,"message":""#.to_vec();
    let fill = 64 * 1024 - body.len() - 1;
    body.extend(std::iter::repeat_n(b'x', fill));
    body.extend_from_slice("€".as_bytes());
    body.extend_from_slice(br#""}"#);
    let server = TestServer::start(vec![
        ScriptedResponse::raw(429, "application/json", Bytes::from(body)),
        ScriptedResponse::json(200, json!({"object": "list", "data": []})),
    ])
    .await;

    let response = FileListRequest::new(FileListPurpose::Batch)
        .send_via(&client_for(&server))
        .await
        .expect("an incomplete diagnostic must not override retryable HTTP 429");

    assert!(response.data.as_deref().is_some_and(|data| data.is_empty()));
    assert_eq!(
        server.requests().len(),
        2,
        "only the HTTP status may control retry after diagnostic truncation"
    );
    server.shutdown().await;
}

#[tokio::test]
async fn oversized_duplicate_diagnostic_is_status_only_and_payload_free() {
    let secret = "private-oversized-duplicate";
    let mut body = format!(r#"{{"code":1113,"code":200,"message":"{secret}","pad":""#).into_bytes();
    body.resize(64 * 1024 + 128, b'x');
    body.extend_from_slice(br#""}"#);
    let server = TestServer::start(vec![ScriptedResponse::raw(
        400,
        "application/json",
        Bytes::from(body),
    )])
    .await;

    let error = FileListRequest::new(FileListPurpose::Batch)
        .send_via(&single_attempt_client_for(&server))
        .await
        .expect_err("an incomplete duplicate envelope must fail closed");

    assert_eq!(error.code(), Some(400));
    assert_eq!(error.category(), ErrorCategory::Client);
    assert_eq!(error.raw_business_code(), None);
    assert_eq!(
        error.message(),
        "HTTP error response body was unavailable or truncated"
    );
    for rendered in [error.to_string(), format!("{error:?}"), error.compact()] {
        assert!(!rendered.contains(secret));
        assert!(!rendered.contains("1113"));
    }
    assert_eq!(server.requests().len(), 1);
    server.shutdown().await;
}

#[tokio::test]
async fn oversized_sse_handshake_error_preserves_http_category() {
    let secret = "private-sse-duplicate";
    let mut body = format!(r#"{{"code":1113,"code":200,"message":"{secret}","pad":""#).into_bytes();
    body.resize(64 * 1024 + 128, b'x');
    body.extend_from_slice(br#""}"#);
    let server = TestServer::start(vec![ScriptedResponse::raw(
        429,
        "application/json",
        Bytes::from(body),
    )])
    .await;

    let result = ChatCompletion::new(GLM5_2 {}, TextMessage::user("hello"))
        .enable_stream()
        .stream_via(&client_for(&server))
        .await;
    let error = match result {
        Ok(_) => panic!("the SSE handshake must not accept an HTTP 429"),
        Err(error) => error,
    };

    assert_eq!(error.code(), Some(429));
    assert_eq!(error.category(), ErrorCategory::RateLimit);
    assert!(error.is_retryable());
    assert_eq!(error.raw_business_code(), None);
    assert_eq!(
        error.message(),
        "HTTP error response body was unavailable or truncated"
    );
    for rendered in [error.to_string(), format!("{error:?}"), error.compact()] {
        assert!(!rendered.contains(secret));
        assert!(!rendered.contains("1113"));
    }
    assert_eq!(server.requests().len(), 1);
    server.shutdown().await;
}

#[tokio::test]
async fn buffered_json_rejects_undeclared_2xx_before_polling_body() {
    for status in [201, 202, 204, 206, 299] {
        let gate = Arc::new(tokio::sync::Semaphore::new(0));
        let response = ScriptedResponse::chunked(
            status,
            "application/json",
            [Bytes::from_static(br#"{"object":"list","data":[]}"#)],
        )
        .with_chunk_gate(gate);
        let server = TestServer::start(vec![response]).await;

        let error = tokio::time::timeout(
            Duration::from_millis(500),
            FileListRequest::new(FileListPurpose::Batch).send_via(&client_for(&server)),
        )
        .await
        .unwrap_or_else(|_| panic!("HTTP {status} attempted to poll its response body"))
        .expect_err("an undeclared 2xx status became typed success");

        assert_eq!(error.code(), Some(codes::SDK_VALIDATION));
        assert_eq!(
            error.message(),
            "response used an undocumented HTTP success status"
        );
        assert_eq!(server.requests().len(), 1);
        server.shutdown().await;
    }
}

#[tokio::test]
async fn buffered_audio_rejects_partial_semantics_before_polling_body() {
    for (status, content_range) in [(206, Some("bytes 0-3/8")), (200, Some("bytes 0-3/8"))] {
        let gate = Arc::new(tokio::sync::Semaphore::new(0));
        let mut response =
            ScriptedResponse::chunked(status, "audio/wav", [Bytes::from_static(b"RIFF")])
                .with_chunk_gate(gate);
        if let Some(value) = content_range {
            response
                .headers
                .push(("content-range".to_owned(), value.to_owned()));
        }
        let server = TestServer::start(vec![response]).await;

        let error = tokio::time::timeout(
            Duration::from_millis(500),
            TextToAudioRequest::new(GlmTts {})
                .with_input("hello")
                .send_via(&single_attempt_client_for(&server)),
        )
        .await
        .unwrap_or_else(|_| panic!("HTTP {status} audio attempted to poll its response body"))
        .expect_err("a partial audio response became complete output");

        assert_eq!(error.code(), Some(codes::SDK_VALIDATION));
        if status == 206 {
            assert_eq!(
                error.message(),
                "response used an undocumented HTTP success status"
            );
        } else {
            assert_eq!(
                error.message(),
                "complete response unexpectedly included Content-Range"
            );
        }
        assert_eq!(server.requests().len(), 1);
        server.shutdown().await;
    }
}

#[tokio::test]
async fn malformed_identifier_diagnostic_never_echoes_provider_payload() {
    let secret = "abc.0123456789abcdef.private-prompt"; // gitleaks:allow -- synthetic test credential
    let server = TestServer::start(vec![ScriptedResponse::raw(
        200,
        "application/json",
        format!(r#"{{"id":{{"echo":"{secret}"}},"choices":[]}}"#),
    )])
    .await;

    let error = ChatCompletion::new(GLM5_2 {}, TextMessage::user("hello"))
        .send_via(&single_attempt_client_for(&server))
        .await
        .expect_err("a wrong-typed response identifier must fail decoding");

    assert!(error.message().contains("expected string, number, or null"));
    for rendered in [error.to_string(), format!("{error:?}"), error.compact()] {
        assert!(!rendered.contains(secret));
        assert!(!rendered.contains("private-prompt"));
    }
    assert_eq!(server.requests().len(), 1);
    server.shutdown().await;
}

#[tokio::test]
async fn malformed_duplicate_codes_use_http_status_only() {
    let non_retryable = TestServer::start(vec![
        ScriptedResponse::raw(400, "application/json", r#"{"code":1302,"code":1113"#),
        ScriptedResponse::json(200, json!({"object": "list", "data": []})),
    ])
    .await;
    let error = FileListRequest::new(FileListPurpose::Batch)
        .send_via(&client_for(&non_retryable))
        .await
        .expect_err("a malformed HTTP 400 diagnostic must not trigger a body-code retry");
    assert_eq!(error.code(), Some(400));
    assert_eq!(error.category(), ErrorCategory::Client);
    assert!(!error.is_retryable());
    assert_eq!(error.raw_business_code(), None);
    assert_eq!(error.message(), "malformed JSON business-error diagnostic");
    assert_eq!(non_retryable.requests().len(), 1);
    non_retryable.shutdown().await;

    let retryable = TestServer::start(vec![
        ScriptedResponse::raw(503, "application/json", r#"{"code":1113,"code":1302"#),
        ScriptedResponse::json(200, json!({"object": "list", "data": []})),
    ])
    .await;
    let response = FileListRequest::new(FileListPurpose::Batch)
        .send_via(&client_for(&retryable))
        .await
        .expect("a malformed diagnostic must not suppress the HTTP 503 retry");
    assert!(response.data.as_deref().is_some_and(|data| data.is_empty()));
    assert_eq!(retryable.requests().len(), 2);
    retryable.shutdown().await;
}

#[tokio::test]
async fn sse_malformed_duplicate_codes_use_http_status_only() {
    for (status, body, category, retryable) in [
        (
            400,
            r#"{"code":1302,"code":1113"#,
            ErrorCategory::Client,
            false,
        ),
        (
            503,
            r#"{"code":1113,"code":1302"#,
            ErrorCategory::Server,
            true,
        ),
    ] {
        let server = TestServer::start(vec![ScriptedResponse::raw(
            status,
            "application/json",
            body,
        )])
        .await;
        let result = ChatCompletion::new(GLM5_2 {}, TextMessage::user("hello"))
            .enable_stream()
            .stream_via(&client_for(&server))
            .await;
        let error = match result {
            Ok(_) => panic!("a malformed HTTP {status} SSE diagnostic became a stream"),
            Err(error) => error,
        };

        assert_eq!(error.code(), Some(status));
        assert_eq!(error.category(), category);
        assert_eq!(error.is_retryable(), retryable);
        assert_eq!(error.raw_business_code(), None);
        assert_eq!(error.message(), "malformed JSON business-error diagnostic");
        assert_eq!(server.requests().len(), 1);
        server.shutdown().await;
    }
}
