//! Integration Tests for zai-rs
//!
//! These tests use a mock server to simulate the Zhipu AI API,
//! allowing for end-to-end testing without making actual API calls.

use std::{
    convert::Infallible,
    sync::{Arc, Mutex},
};

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::{Request, Response, body::Incoming, http::StatusCode, service::service_fn};
use hyper_util::{rt::TokioIo, server::conn::auto::Builder as ConnBuilder};
use serde_json::json;
use tokio::{net::TcpListener, sync::oneshot};
use zai_rs::{
    client::{ApiFamily, ZaiClient},
    file::{FileListPurpose, FileListQuery, FileListRequest, FileUploadPurpose, FileUploadRequest},
    model::{ChatCompletion, GLM5_2, TextMessage},
    usage::CodingPlanUsageRequest,
};

#[derive(Debug)]
struct CapturedHttpRequest {
    method: String,
    path: String,
    query: Option<String>,
    authorization: Option<String>,
    content_type: Option<String>,
    body: Vec<u8>,
}

async fn capture_one_sdk_request(
    response_body: serde_json::Value,
) -> (String, oneshot::Receiver<CapturedHttpRequest>) {
    capture_one_request_under("/api/paas/v4", response_body).await
}

/// Build a `ZaiClient` whose PaasV4 endpoint points at the mock `base_url`
/// so chat requests can be captured without external network access.
fn client_for_mock_base(base_url: &str, key: &str) -> ZaiClient {
    ZaiClient::builder(key)
        .allow_insecure_transport(true)
        .endpoint(ApiFamily::PaasV4, base_url)
        .build()
        .unwrap()
}

/// Build a `ZaiClient` whose Monitor endpoint points at the mock `base_url`
/// so usage requests can be captured without external network access.
fn monitor_client_for_base(base_url: &str, key: &str) -> ZaiClient {
    ZaiClient::builder(key)
        .allow_insecure_transport(true)
        .endpoint(ApiFamily::Monitor, base_url)
        .build()
        .unwrap()
}

#[tokio::test]
async fn test_sdk_json_post_uses_dynamic_mock_base() {
    let key = "test.12345678901234567890".to_string();
    let (base_url, captured) = capture_one_sdk_request(json!({
        "id": "chatcmpl-test",
        "created": 1,
        "model": "glm-5.2",
        "choices": [{
            "index": 0,
            "message": {"role": "assistant", "content": "ok"},
            "finish_reason": "stop"
        }],
        "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2}
    }))
    .await;

    let response = ChatCompletion::new(GLM5_2 {}, TextMessage::user("hello"))
        .send_via(&client_for_mock_base(&base_url, &key))
        .await
        .unwrap();

    assert_eq!(response.model.as_deref(), Some("glm-5.2"));
    let request = captured.await.unwrap();
    assert_eq!(request.method, "POST");
    assert_eq!(request.path, "/api/paas/v4/chat/completions");
    assert_eq!(
        request.authorization.as_deref(),
        Some(format!("Bearer {key}").as_str())
    );
    assert_eq!(request.content_type.as_deref(), Some("application/json"));
    let body: serde_json::Value = serde_json::from_slice(&request.body).unwrap();
    assert_eq!(body["model"], "glm-5.2");
    assert_eq!(body["messages"][0]["content"], "hello");
}

#[tokio::test]
async fn test_sdk_chat_serializes_frozen_tool_choice_and_response_format() {
    use zai_rs::model::tools::{Function, ResponseFormat, ToolChoice, Tools};

    let key = "test.12345678901234567890".to_string();
    let (base_url, captured) = capture_one_sdk_request(json!({
        "id": "chatcmpl-tc",
        "created": 1,
        "model": "glm-5.2",
        "choices": [{
            "index": 0,
            "message": {"role": "assistant", "content": "ok"},
            "finish_reason": "stop"
        }],
        "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2}
    }))
    .await;

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
        .send_via(&client_for_mock_base(&base_url, &key))
        .await
        .unwrap();

    let request = captured.await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&request.body).unwrap();
    assert_eq!(body["tool_choice"], json!("auto"));
    assert_eq!(body["tools"][0]["function"]["name"], "get_weather");
    // response_format reaches the wire too.
    assert_eq!(body["response_format"]["type"], "json_object");
}

#[tokio::test]
async fn test_sdk_chat_uses_configured_coding_plan_base() {
    let key = "test.12345678901234567890".to_string();
    let (coding_base_url, captured) = capture_one_request_under(
        "/api/coding/paas/v4",
        json!({
            "id": "chatcmpl-coding-test",
            "created": 1,
            "model": "glm-5.2",
            "choices": [{
                "index": 0,
                "message": {"role": "assistant", "content": "ok"},
                "finish_reason": "stop"
            }],
            "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2}
        }),
    )
    .await;

    let client = ZaiClient::builder(&key)
        .allow_insecure_transport(true)
        .endpoint(ApiFamily::CodingPaasV4, coding_base_url)
        .build()
        .unwrap();

    let response = ChatCompletion::new(GLM5_2 {}, TextMessage::user("fix this"))
        .send_via_coding_plan(&client)
        .await
        .unwrap();

    assert_eq!(response.model.as_deref(), Some("glm-5.2"));
    let request = captured.await.unwrap();
    assert_eq!(request.method, "POST");
    assert_eq!(request.path, "/api/coding/paas/v4/chat/completions");
    assert_eq!(
        request.authorization.as_deref(),
        Some(format!("Bearer {key}").as_str())
    );
    let body: serde_json::Value = serde_json::from_slice(&request.body).unwrap();
    assert_eq!(body["model"], "glm-5.2");
    assert_eq!(body["messages"][0]["content"], "fix this");
}

#[tokio::test]
async fn test_sdk_get_uses_dynamic_mock_base_and_query() {
    let key = "test.12345678901234567890".to_string();
    let (base_url, captured) = capture_one_sdk_request(json!({
        "object": "list",
        "data": [],
        "has_more": false
    }))
    .await;

    let client = client_for_mock_base(&base_url, &key);
    let response = FileListRequest::new(FileListPurpose::Batch)
        .with_query(FileListQuery::new(FileListPurpose::Batch).with_limit(2))
        .send_via(&client)
        .await
        .unwrap();

    assert_eq!(response.has_more, Some(false));
    let request = captured.await.unwrap();
    assert_eq!(request.method, "GET");
    assert_eq!(request.path, "/api/paas/v4/files");
    assert!(
        request
            .query
            .as_deref()
            .unwrap_or_default()
            .contains("limit=2")
    );
    assert!(
        request
            .query
            .as_deref()
            .unwrap_or_default()
            .contains("purpose=batch")
    );
    assert_eq!(
        request.authorization.as_deref(),
        Some(format!("Bearer {key}").as_str())
    );
    assert!(request.body.is_empty());
}

#[tokio::test]
async fn test_sdk_multipart_uses_dynamic_mock_base() {
    let key = "test.12345678901234567890".to_string();
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path().join("upload.txt");
    std::fs::write(&temp_path, b"hello upload").unwrap();

    let (base_url, captured) = capture_one_sdk_request(json!({
        "id": "file-test",
        "object": "file",
        "bytes": 12,
        "created_at": 1,
        "filename": "sample.txt",
        "purpose": "batch"
    }))
    .await;

    let client = client_for_mock_base(&base_url, &key);
    let response = FileUploadRequest::new(FileUploadPurpose::Batch, &temp_path)
        .with_file_name("sample.txt")
        .with_content_type("text/plain")
        .send_via(&client)
        .await
        .unwrap();

    assert_eq!(response.id.as_deref(), Some("file-test"));
    let request = captured.await.unwrap();
    assert_eq!(request.method, "POST");
    assert_eq!(request.path, "/api/paas/v4/files");
    assert_eq!(
        request.authorization.as_deref(),
        Some(format!("Bearer {key}").as_str())
    );
    assert!(
        request
            .content_type
            .as_deref()
            .unwrap_or_default()
            .starts_with("multipart/form-data; boundary=")
    );
    let body = String::from_utf8_lossy(&request.body);
    assert!(body.contains("name=\"purpose\""));
    assert!(body.contains("batch"));
    assert!(body.contains("filename=\"sample.txt\""));
    assert!(body.contains("hello upload"));
}

/// POST requests are non-idempotent by contract and must not be replayed after
/// a transient server error. This pins the unified transport's retry-safety
/// behavior and guards against duplicate chat submissions.
#[tokio::test]
async fn test_send_path_does_not_retry_non_idempotent_post() {
    use std::sync::atomic::{AtomicU32, Ordering};

    let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
    let base_url = format!("http://{}", listener.local_addr().unwrap());
    let attempts = Arc::new(AtomicU32::new(0));
    let attempts_server = Arc::clone(&attempts);

    tokio::spawn(async move {
        loop {
            let (stream, _) = match listener.accept().await {
                Ok(s) => s,
                Err(_) => break,
            };
            let io = TokioIo::new(stream);
            let counter = Arc::clone(&attempts_server);
            let service = service_fn(move |req: Request<Incoming>| {
                let counter = Arc::clone(&counter);
                async move {
                    // Drain the request body so the connection can be reused.
                    req.collect().await.unwrap();
                    let n = counter.fetch_add(1, Ordering::SeqCst);
                    if n < 2 {
                        // First two attempts: 500 with empty body -> HttpError
                        // (retryable).
                        let mut resp = Response::new(Full::new(Bytes::new()));
                        *resp.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                        Ok::<_, Infallible>(resp)
                    } else {
                        let body = serde_json::to_string(&json!({
                            "id": "chatcmpl-1",
                            "object": "chat.completion",
                            "created": 1,
                            "model": "glm-5.2",
                            "choices": [{
                                "index": 0,
                                "message": {"role": "assistant", "content": "ok"},
                                "finish_reason": "stop"
                            }],
                            "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2}
                        }))
                        .unwrap();
                        Ok(Response::new(Full::new(Bytes::from(body))))
                    }
                }
            });
            ConnBuilder::new(hyper_util::rt::TokioExecutor::new())
                .serve_connection(io, service)
                .await
                .unwrap();
        }
    });

    let key = "test.12345678901234567890".to_string();
    let client = client_for_mock_base(&base_url, &key);
    let resp = ChatCompletion::new(GLM5_2 {}, TextMessage::user("hi"))
        .send_via(&client)
        .await;

    assert!(resp.is_err(), "the first 500 must be returned");
    assert_eq!(
        attempts.load(Ordering::SeqCst),
        1,
        "a non-idempotent POST must be sent exactly once"
    );
}

/// Capture a single SDK request against a mock server bound to `base_path`
/// (e.g. `/api/monitor`). Returns the full base URL the SDK should be
/// configured with and a receiver yielding the captured request.
async fn capture_one_request_under(
    base_path: &str,
    response_body: serde_json::Value,
) -> (String, oneshot::Receiver<CapturedHttpRequest>) {
    let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (tx, rx) = oneshot::channel::<CapturedHttpRequest>();
    let tx = Arc::new(Mutex::new(Some(tx)));

    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let io = TokioIo::new(stream);
        let response_body = response_body.clone();

        let service = service_fn(move |req: Request<Incoming>| {
            let tx = Arc::clone(&tx);
            let response_body = response_body.clone();
            async move {
                let method = req.method().as_str().to_string();
                let uri = req.uri().clone();
                let authorization = req
                    .headers()
                    .get(hyper::header::AUTHORIZATION)
                    .and_then(|value| value.to_str().ok())
                    .map(str::to_string);
                let content_type = req
                    .headers()
                    .get(hyper::header::CONTENT_TYPE)
                    .and_then(|value| value.to_str().ok())
                    .map(str::to_string);
                let body = req.collect().await.unwrap().to_bytes().to_vec();

                if let Some(tx) = tx.lock().unwrap().take() {
                    tx.send(CapturedHttpRequest {
                        method,
                        path: uri.path().to_string(),
                        query: uri.query().map(str::to_string),
                        authorization,
                        content_type,
                        body,
                    })
                    .expect("capture receiver must remain alive until the request arrives");
                }

                let mut response = Response::new(Full::new(Bytes::from(response_body.to_string())));
                *response.status_mut() = StatusCode::OK;
                response.headers_mut().insert(
                    hyper::header::CONTENT_TYPE,
                    hyper::header::HeaderValue::from_static("application/json"),
                );
                Ok::<_, Infallible>(response)
            }
        });

        ConnBuilder::new(hyper_util::rt::TokioExecutor::new())
            .serve_connection(io, service)
            .await
            .unwrap();
    });

    (format!("http://{addr}{base_path}"), rx)
}

/// Coding Plan usage query (GET /api/monitor/usage/quota/limit) hits the
/// monitor endpoint, sends a Bearer token, carries no body, and parses the
/// `{code,msg,success,data}` envelope into typed quota windows.
#[tokio::test]
async fn test_sdk_coding_plan_usage_query() {
    let key = "test.12345678901234567890".to_string();
    let (base_url, captured) = capture_one_request_under(
        "/api/monitor",
        json!({
            "code": 200,
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
        .send_via(&monitor_client_for_base(&base_url, &key))
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
    assert_eq!(summary.code, 200);
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

    let request = captured.await.unwrap();
    assert_eq!(request.method, "GET");
    assert_eq!(request.path, "/api/monitor/usage/quota/limit");
    assert!(request.body.is_empty());
    assert_eq!(
        request.authorization.as_deref(),
        Some(format!("Bearer {key}").as_str())
    );
}
