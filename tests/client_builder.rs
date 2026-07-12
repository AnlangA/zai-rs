//! Integration tests for `ZaiClient` and `ZaiClientBuilder`.
//!
//! These exercise public client composition across secret redaction, endpoint
//! resolution, transport configuration, and outbound header injection.

mod support;
use support::http_server::{ScriptedResponse, TestServer};

use bytes::Bytes;
use zai_rs::client::{AdditionalHeader, ApiFamily, HttpTransportConfig, ZaiClient};

const KEY: &str = "abcdefghij.0123456789abcdef";

fn client_for(server: &TestServer) -> ZaiClient {
    ZaiClient::builder(KEY)
        .allow_insecure_transport(true)
        .endpoint(
            ApiFamily::PaasV4,
            format!("{}/api/paas/v4", server.base_url),
        )
        .build()
        .unwrap()
}

fn request_header(server: &TestServer, name: &str) -> Option<String> {
    server.requests()[0]
        .headers
        .iter()
        .find(|(header, _)| header.eq_ignore_ascii_case(name))
        .map(|(_, value)| value.clone())
}

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
    let base = format!("{}/api/paas/v4", server.base_url);
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
    assert_eq!(requests[0].method, "POST");
    assert_eq!(requests[0].path, "/api/paas/v4/chat/completions");
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
    let body: serde_json::Value = serde_json::from_slice(&requests[0].body).unwrap();
    assert_eq!(body["messages"][0]["content"], "hello");
    server.shutdown().await;
}

#[tokio::test]
async fn json_success_requires_a_json_media_type() {
    let server = TestServer::start(vec![ScriptedResponse::raw(
        200,
        "text/plain",
        Bytes::from_static(br#"{"object":"list","data":[]}"#),
    )])
    .await;
    let client = client_for(&server);

    let error = zai_rs::file::FileListRequest::new(zai_rs::file::FileListPurpose::Batch)
        .send_via(&client)
        .await
        .expect_err("text/plain must not be decoded as a JSON success response");

    assert_eq!(
        error.code(),
        Some(zai_rs::client::error::codes::SDK_VALIDATION)
    );
    assert_eq!(
        request_header(&server, "accept").as_deref(),
        Some("application/json")
    );
    server.shutdown().await;
}

#[tokio::test]
async fn structured_json_suffix_is_accepted() {
    let server = TestServer::start(vec![ScriptedResponse::raw(
        200,
        "application/vnd.zai.result+json; charset=utf-8",
        Bytes::from_static(br#"{"object":"list","data":[]}"#),
    )])
    .await;
    let client = client_for(&server);

    let response = zai_rs::file::FileListRequest::new(zai_rs::file::FileListPurpose::Batch)
        .send_via(&client)
        .await
        .unwrap();

    assert_eq!(response.data.map_or(0, |items| items.len()), 0);
    server.shutdown().await;
}

#[tokio::test]
async fn file_and_audio_routes_use_distinct_media_contracts() {
    use zai_rs::{
        file::FileContentRequest,
        model::text_to_audio::{GlmTts, TextToAudioRequest},
    };

    let file_server = TestServer::start(vec![ScriptedResponse::raw(
        200,
        "audio/wav",
        Bytes::from_static(b"not-a-file-response"),
    )])
    .await;
    let file_client = client_for(&file_server);
    let error = FileContentRequest::new("file-1")
        .send_via(&file_client)
        .await
        .expect_err("file downloads must require application/octet-stream");
    assert_eq!(
        error.code(),
        Some(zai_rs::client::error::codes::SDK_VALIDATION)
    );
    assert_eq!(
        request_header(&file_server, "accept").as_deref(),
        Some("application/octet-stream")
    );
    file_server.shutdown().await;

    let audio_server = TestServer::start(vec![ScriptedResponse::raw(
        200,
        "audio/mpeg",
        Bytes::from_static(b"unsupported-audio"),
    )])
    .await;
    let audio_client = client_for(&audio_server);
    let error = TextToAudioRequest::new(GlmTts {})
        .with_input("hello")
        .send_via(&audio_client)
        .await
        .expect_err("undocumented TTS media types must be rejected");
    assert_eq!(
        error.code(),
        Some(zai_rs::client::error::codes::SDK_VALIDATION)
    );
    assert_eq!(
        request_header(&audio_server, "accept").as_deref(),
        Some("audio/wav, audio/x-wav, audio/pcm, application/octet-stream")
    );
    audio_server.shutdown().await;
}
