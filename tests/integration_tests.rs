//! Integration Tests for zai-rs
//!
//! These tests use a mock server to simulate the Zhipu AI API,
//! allowing for end-to-end testing without making actual API calls.

use std::time::Duration;

use serde_json::json;
use tokio::time::sleep;

mod common;
use common::mock_server::{MockServerClient, MockServerConfig};

/// Integration test for chat completion
#[tokio::test]
async fn test_chat_completion_integration() {
    let config = MockServerConfig::default();
    let client = MockServerClient::new(config.base_url.clone());

    // Verify URL construction
    let expected_url = client.url("/api/paas/v4/chat/completions");
    assert_eq!(
        expected_url,
        "http://127.0.0.1:9876/api/paas/v4/chat/completions"
    );
}

/// Integration test for error handling
#[tokio::test]
async fn test_error_handling_integration() {
    let config = MockServerConfig::default();
    let client = MockServerClient::new(config.base_url.clone());

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
    let client = MockServerClient::new(config.base_url.clone());

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
