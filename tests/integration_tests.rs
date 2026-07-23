//! Integration Tests for zai-rs
//!
//! These tests use the shared scripted mock server to simulate the Zhipu AI
//! API, allowing for end-to-end testing without making actual API calls.

mod support;

use serde_json::json;
use support::http_server::{CapturedRequest, ScriptedResponse, TestServer};

use zai_rs::{
    client::{ApiFamily, ZaiClient},
    file::{FileListPurpose, FileListQuery, FileListRequest, FileUploadPurpose, FileUploadRequest},
    model::{ChatCompletion, GLM5_2, TextMessage},
    usage::CodingPlanUsageRequest,
};

const KEY: &str = "test.12345678901234567890";

/// Build a `ZaiClient` whose `family` endpoint points at the mock `base_url`
/// so requests can be captured without external network access.
fn client_for(family: ApiFamily, base_url: &str) -> ZaiClient {
    ZaiClient::builder(KEY)
        .allow_insecure_transport(true)
        .endpoint(family, base_url)
        .build()
        .unwrap()
}

/// Start a mock server serving one JSON 200 response and return it together
/// with the full base URL (including `base_path`) the SDK should point at.
async fn server_under(base_path: &str, body: serde_json::Value) -> (TestServer, String) {
    let server = TestServer::start(vec![ScriptedResponse::json(200, body)]).await;
    let base = format!("{}{base_path}", server.base_url);
    (server, base)
}

/// Start a mock server for the PaasV4 API family.
async fn paas_server(body: serde_json::Value) -> (TestServer, String) {
    server_under("/api/paas/v4", body).await
}

/// The single request the server must have captured.
fn only_request(server: &TestServer) -> CapturedRequest {
    let requests = server.requests();
    assert_eq!(requests.len(), 1, "expected exactly one HTTP request");
    requests.into_iter().next().unwrap()
}

/// Look up a captured header value by name.
fn header<'a>(request: &'a CapturedRequest, name: &str) -> Option<&'a str> {
    request
        .headers
        .iter()
        .find(|(k, _)| k == name)
        .map(|(_, v)| v.as_str())
}

fn chat_ok_body() -> serde_json::Value {
    json!({
        "id": "chatcmpl-test",
        "created": 1,
        "model": "glm-5.2",
        "choices": [{
            "index": 0,
            "message": {"role": "assistant", "content": "ok"},
            "finish_reason": "stop"
        }],
        "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2}
    })
}

#[tokio::test]
async fn test_sdk_json_post_uses_dynamic_mock_base() {
    let (server, base_url) = paas_server(chat_ok_body()).await;

    let response = ChatCompletion::new(GLM5_2 {}, TextMessage::user("hello"))
        .send_via(&client_for(ApiFamily::PaasV4, &base_url))
        .await
        .unwrap();

    assert_eq!(response.model.as_deref(), Some("glm-5.2"));
    let request = only_request(&server);
    assert_eq!(request.method, "POST");
    assert_eq!(request.path, "/api/paas/v4/chat/completions");
    assert_eq!(
        request.authorization.as_deref(),
        Some(format!("Bearer {KEY}").as_str())
    );
    assert_eq!(header(&request, "content-type"), Some("application/json"));
    let body: serde_json::Value = serde_json::from_slice(&request.body).unwrap();
    assert_eq!(body["model"], "glm-5.2");
    assert_eq!(body["messages"][0]["content"], "hello");
    server.shutdown().await;
}

#[tokio::test]
async fn test_sdk_chat_serializes_frozen_tool_choice_and_response_format() {
    use zai_rs::model::tools::{Function, ResponseFormat, ToolChoice, Tools};

    let (server, base_url) = paas_server(chat_ok_body()).await;

    ChatCompletion::new(GLM5_2 {}, TextMessage::user("hi"))
        .add_tool(Tools::Function {
            function: Function::new(
                "get_weather",
                "Get current weather",
                json!({"type": "object"}),
            ),
        })
        .with_tool_choice(ToolChoice::auto())
        .with_response_format(ResponseFormat::JsonObject)
        .send_via(&client_for(ApiFamily::PaasV4, &base_url))
        .await
        .unwrap();

    let request = only_request(&server);
    let body: serde_json::Value = serde_json::from_slice(&request.body).unwrap();
    assert_eq!(body["tool_choice"], json!("auto"));
    assert_eq!(body["tools"][0]["function"]["name"], "get_weather");
    // response_format reaches the wire too.
    assert_eq!(body["response_format"]["type"], "json_object");
    server.shutdown().await;
}

#[tokio::test]
async fn test_sdk_chat_uses_configured_coding_plan_base() {
    let (server, coding_base_url) = server_under("/api/coding/paas/v4", chat_ok_body()).await;

    let response = ChatCompletion::new(GLM5_2 {}, TextMessage::user("fix this"))
        .send_via_coding_plan(&client_for(ApiFamily::CodingPaasV4, &coding_base_url))
        .await
        .unwrap();

    assert_eq!(response.model.as_deref(), Some("glm-5.2"));
    let request = only_request(&server);
    assert_eq!(request.method, "POST");
    assert_eq!(request.path, "/api/coding/paas/v4/chat/completions");
    assert_eq!(
        request.authorization.as_deref(),
        Some(format!("Bearer {KEY}").as_str())
    );
    let body: serde_json::Value = serde_json::from_slice(&request.body).unwrap();
    assert_eq!(body["model"], "glm-5.2");
    assert_eq!(body["messages"][0]["content"], "fix this");
    server.shutdown().await;
}

#[tokio::test]
async fn test_sdk_get_uses_dynamic_mock_base_and_query() {
    let (server, base_url) = paas_server(json!({
        "object": "list",
        "data": [],
        "has_more": false
    }))
    .await;

    let response = FileListRequest::new(FileListPurpose::Batch)
        .with_query(FileListQuery::new(FileListPurpose::Batch).with_limit(2))
        .send_via(&client_for(ApiFamily::PaasV4, &base_url))
        .await
        .unwrap();

    assert_eq!(response.has_more, Some(false));
    let request = only_request(&server);
    assert_eq!(request.method, "GET");
    assert_eq!(request.path, "/api/paas/v4/files");
    let query = request.query.as_deref().unwrap_or_default();
    assert!(query.contains("limit=2"));
    assert!(query.contains("purpose=batch"));
    assert_eq!(
        request.authorization.as_deref(),
        Some(format!("Bearer {KEY}").as_str())
    );
    assert!(request.body.is_empty());
    server.shutdown().await;
}

#[tokio::test]
async fn test_sdk_multipart_uses_dynamic_mock_base() {
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path().join("upload.txt");
    std::fs::write(&temp_path, b"hello upload").unwrap();

    let (server, base_url) = paas_server(json!({
        "id": "file-test",
        "object": "file",
        "bytes": 12,
        "created_at": 1,
        "filename": "sample.txt",
        "purpose": "batch"
    }))
    .await;

    let response = FileUploadRequest::new(FileUploadPurpose::Batch, &temp_path)
        .with_file_name("sample.txt")
        .with_content_type("text/plain")
        .send_via(&client_for(ApiFamily::PaasV4, &base_url))
        .await
        .unwrap();

    assert_eq!(response.id.as_deref(), Some("file-test"));
    let request = only_request(&server);
    assert_eq!(request.method, "POST");
    assert_eq!(request.path, "/api/paas/v4/files");
    assert_eq!(
        request.authorization.as_deref(),
        Some(format!("Bearer {KEY}").as_str())
    );
    assert!(
        header(&request, "content-type")
            .unwrap_or_default()
            .starts_with("multipart/form-data; boundary=")
    );
    let body = String::from_utf8_lossy(&request.body);
    assert!(body.contains("name=\"purpose\""));
    assert!(body.contains("batch"));
    assert!(body.contains("filename=\"sample.txt\""));
    assert!(body.contains("hello upload"));
    server.shutdown().await;
}

/// POST requests are non-idempotent by contract and must not be replayed after
/// a transient server error. This pins the unified transport's retry-safety
/// behavior and guards against duplicate chat submissions.
#[tokio::test]
async fn test_send_path_does_not_retry_non_idempotent_post() {
    let server = TestServer::start(vec![
        ScriptedResponse::empty(500),
        ScriptedResponse::empty(500),
        ScriptedResponse::json(200, chat_ok_body()),
    ])
    .await;
    let base_url = format!("{}/api/paas/v4", server.base_url);

    let resp = ChatCompletion::new(GLM5_2 {}, TextMessage::user("hi"))
        .send_via(&client_for(ApiFamily::PaasV4, &base_url))
        .await;

    assert!(resp.is_err(), "the first 500 must be returned");
    assert_eq!(
        server.requests().len(),
        1,
        "a non-idempotent POST must be sent exactly once"
    );
    server.shutdown().await;
}

/// Coding Plan usage query (GET /api/monitor/usage/quota/limit) hits the
/// monitor endpoint, sends a Bearer token, carries no body, and parses the
/// `{code,msg,success,data}` envelope into typed quota windows.
#[tokio::test]
async fn test_sdk_coding_plan_usage_query() {
    let (server, base_url) = server_under(
        "/api/monitor",
        json!({
            "code": 0,
            "msg": "ok",
            "success": true,
            "data": {
                "level": 3,
                "limits": [
                    {
                        "type": "TIME_LIMIT",
                        "unit": 5,
                        "number": 600,
                        "percentage": 25.0,
                        "usage": 4000,
                        "currentValue": 54,
                        "remaining": 3946,
                        "nextResetTime": 1781778751996_i64,
                        "usageDetails": [
                            {"modelCode": "search-prime", "usage": 40},
                            {"modelCode": "web-reader", "usage": 14}
                        ]
                    },
                    {
                        "type": "TOKENS_LIMIT",
                        "unit": 3,
                        "number": 1000000,
                        "percentage": 50.0,
                        "nextResetTime": 1784339999983_i64
                    }
                ]
            }
        }),
    )
    .await;

    let response = CodingPlanUsageRequest::new()
        .send_via(&client_for(ApiFamily::Monitor, &base_url))
        .await
        .unwrap();

    assert!(response.success);
    assert_eq!(response.level(), Some("3"));

    let five_hour = response.time_limit().expect("time limit window present");
    assert!(five_hour.is_time_limit());
    assert_eq!(five_hour.unit.as_deref(), Some("5"));
    assert_eq!(five_hour.quota(), 4000);
    assert_eq!(five_hour.consumed(), 54);
    assert_eq!(five_hour.remaining(), 3946);
    assert_eq!(five_hour.usage_details.len(), 2);
    assert_eq!(five_hour.next_reset_time.as_deref(), Some("1781778751996"));

    let summary = response.summary();
    assert_eq!(summary.code, 0);
    assert_eq!(summary.msg.as_deref(), Some("ok"));
    assert!(summary.success);
    let summarized_time = summary.time_limit().expect("time limit summary present");
    assert_eq!(summarized_time.number, 600);
    assert_eq!(summarized_time.reported_usage, Some(4000));
    assert_eq!(summarized_time.current_value, Some(54));
    assert_eq!(summarized_time.reported_remaining, Some(3946));
    assert_eq!(summarized_time.used, 54);
    assert_eq!(summarized_time.remaining, 3946);
    assert_eq!(
        summarized_time.next_reset_at.as_ref().unwrap().to_rfc3339(),
        "2026-06-18T10:32:31.996+00:00"
    );

    let weekly = response
        .tokens_limit()
        .expect("tokens limit window present");
    assert!(weekly.is_tokens_limit());
    assert_eq!(weekly.remaining(), 500_000);

    let request = only_request(&server);
    assert_eq!(request.method, "GET");
    assert_eq!(request.path, "/api/monitor/usage/quota/limit");
    assert!(request.body.is_empty());
    assert_eq!(
        request.authorization.as_deref(),
        Some(format!("Bearer {KEY}").as_str())
    );
    server.shutdown().await;
}
