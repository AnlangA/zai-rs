//! Integration tests for `ZaiClient` and `ZaiClientBuilder`.
//!
//! These exercise the builder invariants (blank-key rejection, validated
//! endpoints, insecure-loopback, additional-header allow-list, secret redaction)
//! and the local [`TestServer`] for observing an outbound request's headers.

mod support;
use support::http_server::{ScriptedResponse, TestServer};

use std::time::Duration;

use bytes::Bytes;
use zai_rs::client::{AdditionalHeader, ApiFamily, HttpTransportConfig, RetryOverride, ZaiClient};

const KEY: &str = "abcdefghij.0123456789abcdef";

#[tokio::test]
async fn builder_constructs_client_and_redacts_secret() {
    let client = ZaiClient::builder(KEY).build().unwrap();
    // The client's Debug output must never contain the key; the secret field is
    // redacted.
    let dbg = format!("{client:?}");
    assert!(dbg.contains("[REDACTED]"));
    assert!(!dbg.contains("abcdefghij"));
    // Default endpoints resolve to the official bases.
    let url = client
        .endpoints()
        .resolve(ApiFamily::PaasV4, &["chat", "completions"])
        .unwrap();
    assert!(url.ends_with("/api/paas/v4/chat/completions"));
}

#[test]
fn builder_rejects_blank_key() {
    assert!(ZaiClient::builder("").build().is_err());
    assert!(ZaiClient::builder("   ").build().is_err());
}

#[test]
fn builder_rejects_public_http_without_insecure() {
    let res = ZaiClient::builder(KEY)
        .endpoint(ApiFamily::PaasV4, "http://open.bigmodel.cn/api/paas/v4")
        .build();
    assert!(
        res.is_err(),
        "public HTTP must be rejected without insecure"
    );
}

#[test]
fn builder_allows_http_loopback_with_insecure() {
    let client = ZaiClient::builder(KEY)
        .allow_insecure_transport(true)
        .endpoint(ApiFamily::PaasV4, "http://127.0.0.1:8080/api/paas/v4")
        .build()
        .unwrap();
    let url = client.endpoints().resolve(ApiFamily::PaasV4, &[]).unwrap();
    assert!(url.starts_with("http://127.0.0.1:8080/"));
}

#[test]
fn additional_header_disallows_auth_cookie_proxy() {
    assert!(AdditionalHeader::new("Authorization", "x").is_err());
    assert!(AdditionalHeader::new("Cookie", "x").is_err());
    assert!(AdditionalHeader::new("Proxy-Authorization", "x").is_err());
    assert!(AdditionalHeader::new("X-Test-Client", "preserved").is_ok());
}

#[test]
fn transport_only_allows_lowering() {
    let t = HttpTransportConfig::builder()
        .request_timeout(Duration::from_secs(5))
        .unwrap()
        .max_attempts(2)
        .unwrap()
        .build();
    assert_eq!(t.request_timeout, Duration::from_secs(5));
    assert_eq!(t.max_attempts, 2);
    // Raising is rejected.
    assert!(
        HttpTransportConfig::default()
            .with_request_timeout(Duration::from_secs(120))
            .is_err()
    );
}

#[test]
fn retry_override_is_constructible() {
    // Pin that the escape hatch exists and is the only variant.
    let _o = RetryOverride::AssumeIdempotent;
    match _o {
        RetryOverride::AssumeIdempotent => {},
    }
}

#[tokio::test]
async fn test_server_serves_scripted_response_and_captures_request() {
    // Smoke-test the TestServer itself: a 200 JSON body is served and the
    // request is captured. Later transport tests build on this.
    let server = TestServer::start(vec![ScriptedResponse::json(
        200,
        serde_json::json!({"ok": true}),
    )])
    .await;

    let resp = reqwest::Client::new()
        .post(format!("{}/api/paas/v4/chat/completions", server.base_url))
        .header("authorization", "Bearer test")
        .header("content-type", "application/json")
        .body(Bytes::from_static(b"{}"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let reqs = server.requests();
    assert_eq!(reqs.len(), 1);
    assert_eq!(reqs[0].method, "POST");
    assert_eq!(reqs[0].path, "/api/paas/v4/chat/completions");
    assert_eq!(reqs[0].authorization.as_deref(), Some("Bearer test"));

    server.shutdown().await;
}

#[tokio::test]
async fn unified_transport_injects_auth_and_additional_headers() {
    let server = TestServer::start(vec![ScriptedResponse::json(
        200,
        serde_json::json!({
            "id": "chatcmpl-1",
            "model": "glm-5.2",
            "choices": [{
                "index": 0,
                "message": {"role": "assistant", "content": "ok"},
                "finish_reason": "stop"
            }],
            "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2}
        }),
    )])
    .await;
    let base: &'static str = Box::leak(format!("{}/api/paas/v4", server.base_url).into_boxed_str());
    let transport = HttpTransportConfig::builder()
        .additional_header(AdditionalHeader::new("X-Test-Client", "preserved").unwrap())
        .build();
    let client = ZaiClient::builder(KEY)
        .allow_insecure_transport(true)
        .endpoint(ApiFamily::PaasV4, base)
        .transport(transport)
        .build()
        .unwrap();

    use zai_rs::model::{ChatCompletion, GLM5_2, TextMessage};
    ChatCompletion::new(GLM5_2 {}, TextMessage::user("hello"))
        .send_via(&client)
        .await
        .unwrap();

    let requests = server.requests();
    assert_eq!(requests.len(), 1);
    let expected_auth = format!("Bearer {KEY}");
    assert_eq!(
        requests[0].authorization.as_deref(),
        Some(expected_auth.as_str())
    );
    assert!(
        requests[0]
            .headers
            .iter()
            .any(|(name, value)| name == "x-test-client" && value == "preserved")
    );
    server.shutdown().await;
}
