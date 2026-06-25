//! Integration Tests for zai-rs
//!
//! These tests use a mock server to simulate the Zhipu AI API,
//! allowing for end-to-end testing without making actual API calls.

use std::{
    convert::Infallible,
    sync::{Arc, Mutex},
    time::Duration,
};

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::{Request, Response, body::Incoming, http::StatusCode, service::service_fn};
use hyper_util::{rt::TokioIo, server::conn::auto::Builder as ConnBuilder};
use serde_json::json;
use tokio::{net::TcpListener, sync::oneshot, time::sleep};
use zai_rs::{
    client::EndpointConfig,
    file::{FileListQuery, FileListRequest, FilePurpose, FileUploadRequest},
    model::{ChatCompletion, GLM5_2, TextMessage},
    usage::CodingPlanUsageRequest,
};

mod common;
use common::mock_server::{MockServerClient, MockServerConfig};

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
                    let _ = tx.send(CapturedHttpRequest {
                        method,
                        path: uri.path().to_string(),
                        query: uri.query().map(str::to_string),
                        authorization,
                        content_type,
                        body,
                    });
                }

                let mut response = Response::new(Full::new(Bytes::from(response_body.to_string())));
                *response.status_mut() = StatusCode::OK;
                Ok::<_, Infallible>(response)
            }
        });

        ConnBuilder::new(hyper_util::rt::TokioExecutor::new())
            .serve_connection(io, service)
            .await
            .unwrap();
    });

    (format!("http://{addr}/api/paas/v4"), rx)
}

/// Integration test for chat completion
#[tokio::test]
async fn test_chat_completion_integration() {
    let config = MockServerConfig::default();
    let client = MockServerClient::new(config.base_url);

    // Verify URL construction
    let expected_url = client.url("/api/paas/v4/chat/completions");
    assert_eq!(
        expected_url,
        "http://127.0.0.1:9876/api/paas/v4/chat/completions"
    );
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

    let response = ChatCompletion::new(GLM5_2 {}, TextMessage::user("hello"), key.clone())
        .with_base_url(base_url)
        .send()
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
async fn test_sdk_chat_serializes_tool_choice_and_response_format() {
    use zai_rs::model::tools::{ResponseFormat, ToolChoice};

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

    let _ = ChatCompletion::new(GLM5_2 {}, TextMessage::user("hi"), key.clone())
        .with_base_url(base_url)
        .with_tool_choice(ToolChoice::function("get_weather"))
        .with_response_format(ResponseFormat::JsonObject)
        .send()
        .await
        .unwrap();

    let request = captured.await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&request.body).unwrap();
    // tool_choice reaches the wire in the documented object form.
    assert_eq!(
        body["tool_choice"],
        json!({"type": "function", "function": {"name": "get_weather"}})
    );
    // response_format reaches the wire too.
    assert_eq!(body["response_format"]["type"], "json_object");
}

#[tokio::test]
async fn test_sdk_chat_tool_choice_auto_serializes_as_bare_string() {
    use zai_rs::model::tools::ToolChoice;

    let key = "test.12345678901234567890".to_string();
    let (base_url, captured) = capture_one_sdk_request(json!({
        "id": "chatcmpl-tc-auto",
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

    let _ = ChatCompletion::new(GLM5_2 {}, TextMessage::user("hi"), key.clone())
        .with_base_url(base_url)
        .with_tool_choice(ToolChoice::auto())
        .send()
        .await
        .unwrap();

    let request = captured.await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&request.body).unwrap();
    // auto must serialize as a bare JSON string, not an object.
    assert_eq!(body["tool_choice"], json!("auto"));
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

    let endpoint_config = EndpointConfig::default().with_coding_paas_v4_base(coding_base_url);

    let response = ChatCompletion::new(GLM5_2 {}, TextMessage::user("fix this"), key.clone())
        .with_endpoint_config(endpoint_config)
        .with_coding_plan()
        .send()
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

    let response = FileListRequest::new(key.clone())
        .with_query(
            FileListQuery::new()
                .with_purpose(FilePurpose::Batch)
                .with_limit(2),
        )
        .with_base_url(base_url)
        .send()
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
    let temp_path = std::env::temp_dir().join(format!(
        "zai-rs-upload-{}-{}.txt",
        std::process::id(),
        chrono::Utc::now().timestamp_nanos_opt().unwrap()
    ));
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

    let response = FileUploadRequest::new(key.clone(), FilePurpose::Batch, &temp_path)
        .with_base_url(base_url)
        .with_file_name("sample.txt")
        .with_content_type("text/plain")
        .send()
        .await
        .unwrap();

    std::fs::remove_file(&temp_path).unwrap();

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

/// Integration test for error handling
#[tokio::test]
async fn test_error_handling_integration() {
    let config = MockServerConfig::default();
    let client = MockServerClient::new(config.base_url);

    // Test URL construction for different endpoints
    let embeddings_url = client.url("/api/paas/v4/embeddings");
    assert_eq!(
        embeddings_url,
        "http://127.0.0.1:9876/api/paas/v4/embeddings"
    );

    let files_url = client.url("/api/paas/v4/files/file-123");
    assert_eq!(
        files_url,
        "http://127.0.0.1:9876/api/paas/v4/files/file-123"
    );
}

/// Integration test for file operations
#[tokio::test]
async fn test_file_operations_integration() {
    let config = MockServerConfig::default();
    let client = MockServerClient::new(config.base_url);

    // Test file URL construction
    let file_url = client.url("/api/paas/v4/files/file-123456");
    assert_eq!(
        file_url,
        "http://127.0.0.1:9876/api/paas/v4/files/file-123456"
    );
}

/// Integration test for API key authentication simulation
#[tokio::test]
async fn test_api_key_authentication() {
    let config = MockServerConfig::default();
    assert!(!config.api_key.is_empty());
    assert!(config.api_key.contains('.'));
}

/// Test request serialization
#[test]
fn test_request_serialization() {
    let request_body = json!({
        "model": "glm-4",
        "messages": [{"role": "user", "content": "Hello"}],
        "temperature": 0.7
    });

    let serialized = serde_json::to_string(&request_body).unwrap();
    assert!(serialized.contains("glm-4"));
    assert!(serialized.contains("Hello"));
}

/// Test response deserialization
#[test]
fn test_response_deserialization() {
    let response_body = json!({
        "id": "chatcmpl-123",
        "object": "chat.completion",
        "created": 1704067200,
        "model": "glm-4",
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": "Test response"
            },
            "finish_reason": "stop"
        }],
        "usage": {
            "prompt_tokens": 10,
            "completion_tokens": 5,
            "total_tokens": 15
        }
    });

    let response: serde_json::Value = serde_json::from_str(&response_body.to_string()).unwrap();
    assert_eq!(response["model"], "glm-4");
    assert_eq!(
        response["choices"][0]["message"]["content"],
        "Test response"
    );
}

/// Test error response parsing
#[test]
fn test_error_response_parsing() {
    let error_body = json!({
        "error": {
            "code": 1001,
            "message": "Invalid API key"
        }
    });

    let error: serde_json::Value = serde_json::from_str(&error_body.to_string()).unwrap();
    assert_eq!(error["error"]["code"], 1001);
    assert_eq!(error["error"]["message"], "Invalid API key");
}

/// Test streaming response format
#[test]
fn test_streaming_response_format() {
    let stream_chunk = json!({
        "id": "chatcmpl-123",
        "object": "chat.completion.chunk",
        "created": 1704067200,
        "model": "glm-4",
        "choices": [{
            "index": 0,
            "delta": {
                "content": "Hello"
            }
        }]
    });

    let chunk: serde_json::Value = serde_json::from_str(&stream_chunk.to_string()).unwrap();
    assert_eq!(chunk["choices"][0]["delta"]["content"], "Hello");
}

/// Test timeout handling
#[tokio::test]
async fn test_timeout_handling() {
    let start = std::time::Instant::now();

    // Simulate a timeout scenario
    sleep(Duration::from_millis(100)).await;

    let elapsed = start.elapsed();
    assert!(elapsed >= Duration::from_millis(100));
}

/// Test concurrent request handling
#[tokio::test]
async fn test_concurrent_requests() {
    let config = MockServerConfig::default();
    let base_url = config.base_url.clone();
    let client = MockServerClient::new(base_url.clone());

    // Clone URLs for each task
    let url1 = base_url.clone();
    let url2 = base_url.clone();
    let _url3 = base_url.clone();

    // Simulate multiple concurrent requests
    let handles = vec![
        tokio::spawn(async move {
            sleep(Duration::from_millis(10)).await;
            client.url("/api/paas/v4/chat/completions")
        }),
        tokio::spawn(async move {
            let client = MockServerClient::new(url1);
            sleep(Duration::from_millis(10)).await;
            client.url("/api/paas/v4/embeddings")
        }),
        tokio::spawn(async move {
            let client = MockServerClient::new(url2);
            sleep(Duration::from_millis(10)).await;
            client.url("/api/paas/v4/files")
        }),
    ];

    let results = futures::future::join_all(handles).await;
    assert_eq!(results.len(), 3);
}

/// Test retry mechanism simulation
#[tokio::test]
async fn test_retry_simulation() {
    let mut retry_count = 0;
    let max_retries = 3;

    while retry_count < max_retries {
        // Simulate a request that fails twice then succeeds
        retry_count += 1;

        if retry_count < max_retries {
            sleep(Duration::from_millis(10)).await;
            continue;
        }

        break;
    }

    assert_eq!(retry_count, 3);
}

/// End-to-end exercise of the SDK's real retry loop
/// (`send_with_retry_factory`): a server that responds `500` (retryable as
/// `HttpError`) twice and then `200` must be driven through exactly three
/// attempts and ultimately succeed. Uses `RetryDelay::None` so the backoff
/// sleeps are instant. The error body is intentionally empty so the response
/// classifies as `HttpError { 500 }` (retryable), not an unmapped business
/// code (`Unknown`, which is deliberately non-retryable).
#[tokio::test]
async fn test_send_path_retries_500_then_succeeds() {
    use std::sync::atomic::{AtomicU32, Ordering};
    use zai_rs::client::http::{HttpClientConfig, RetryDelay};

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
                    let _ = req.collect().await.unwrap().to_bytes();
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
            let _ = ConnBuilder::new(hyper_util::rt::TokioExecutor::new())
                .serve_connection(io, service)
                .await;
        }
    });

    let cfg = HttpClientConfig::builder()
        .max_retries(2)
        .retry_delay(RetryDelay::None)
        .build();
    let key = "test.12345678901234567890".to_string();
    let resp = ChatCompletion::new(GLM5_2 {}, TextMessage::user("hi"), key)
        .with_base_url(base_url)
        .with_http_config(cfg)
        .send()
        .await;

    assert!(
        resp.is_ok(),
        "should succeed after retries: {:?}",
        resp.err()
    );
    assert_eq!(
        attempts.load(Ordering::SeqCst),
        3,
        "expected exactly 3 attempts (1 initial + 2 retries)"
    );
}

/// Test request validation
#[test]
fn test_request_validation() {
    // Valid request
    let valid_request = json!({
        "model": "glm-4",
        "messages": [{"role": "user", "content": "Test"}]
    });

    assert!(valid_request["model"].is_string());
    assert!(valid_request["messages"].is_array());
    assert!(!valid_request["messages"].as_array().unwrap().is_empty());
}

/// Test response validation
#[test]
fn test_response_validation() {
    let valid_response = json!({
        "id": "chatcmpl-123",
        "choices": [{
            "message": {"content": "Response"}
        }],
        "usage": {"total_tokens": 100}
    });

    assert!(valid_response["id"].is_string());
    assert!(valid_response["choices"].is_array());
    assert!(valid_response["usage"]["total_tokens"].is_number());
}

/// Test large payload handling
#[test]
fn test_large_payload_handling() {
    // Create a large message
    let large_content = "x".repeat(10000);
    let large_request = json!({
        "model": "glm-4",
        "messages": [{"role": "user", "content": large_content}]
    });

    let serialized = serde_json::to_string(&large_request).unwrap();
    assert!(serialized.len() > 10000);
}

/// Test empty response handling
#[test]
fn test_empty_response_handling() {
    let empty_response = json!({
        "id": "chatcmpl-123",
        "choices": [],
        "usage": {"total_tokens": 0}
    });

    assert!(empty_response["choices"].as_array().unwrap().is_empty());
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
                let body = req.collect().await.unwrap().to_bytes().to_vec();

                if let Some(tx) = tx.lock().unwrap().take() {
                    let _ = tx.send(CapturedHttpRequest {
                        method,
                        path: uri.path().to_string(),
                        query: uri.query().map(str::to_string),
                        authorization,
                        content_type: None,
                        body,
                    });
                }

                let mut response = Response::new(Full::new(Bytes::from(response_body.to_string())));
                *response.status_mut() = StatusCode::OK;
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

    let response = CodingPlanUsageRequest::new(key.clone())
        .with_base_url(base_url)
        .send()
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
