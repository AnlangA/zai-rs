//! Mock Server for Integration Tests
//!
//! This module provides a mock HTTP server that simulates the Zhipu AI API
//! for integration testing purposes.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use hyper::{
    Body, Request, Response, Server, StatusCode,
    header::AUTHORIZATION,
    service::{make_service_fn, service_fn},
};
use serde_json::json;

/// Mock server configuration
#[derive(Debug, Clone)]
pub struct MockServerConfig {
    /// API key for authentication
    pub api_key: String,
    /// Base URL for the mock server
    pub base_url: String,
}

impl Default for MockServerConfig {
    fn default() -> Self {
        Self {
            api_key: "test.12345678901234567890".to_string(),
            base_url: "http://127.0.0.1:9876".to_string(),
        }
    }
}

/// Mock server state
#[derive(Debug, Clone)]
pub struct MockServerState {
    _config: MockServerConfig,
    responses: Arc<Mutex<HashMap<String, serde_json::Value>>>,
}

impl MockServerState {
    /// Create a new mock server state
    pub fn new(config: MockServerConfig) -> Self {
        Self {
            _config: config,
            responses: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Register a custom response for a specific endpoint
    pub fn register_response(&self, endpoint: &str, response: serde_json::Value) {
        let mut responses = self.responses.lock().unwrap();
        responses.insert(endpoint.to_string(), response);
    }

    /// Get a registered response for an endpoint
    pub fn get_response(&self, endpoint: &str) -> Option<serde_json::Value> {
        let responses = self.responses.lock().unwrap();
        responses.get(endpoint).cloned()
    }
}

/// Start the mock server
#[allow(dead_code)]
pub async fn start_mock_server(config: MockServerConfig) -> Result<(), Box<dyn std::error::Error>> {
    let state = MockServerState::new(config.clone());
    let addr = ([127, 0, 0, 1], 9876).into();

    let make_svc = make_service_fn(move |_| {
        let state = state.clone();
        async move {
            Ok::<_, hyper::Error>(service_fn(move |req| {
                let state = state.clone();
                async move { handle_request(req, state).await }
            }))
        }
    });

    let server = Server::bind(&addr).serve(make_svc);
    println!("Mock server running on http://127.0.0.1:9876");
    server.await?;
    Ok(())
}

/// Handle incoming requests
#[allow(dead_code)]
async fn handle_request(
    req: Request<Body>,
    state: MockServerState,
) -> Result<Response<Body>, hyper::Error> {
    let path = req.uri().path();
    let method = req.method().as_str();

    // Verify authentication
    let auth_header = req
        .headers()
        .get(AUTHORIZATION)
        .and_then(|h| h.to_str().ok());

    if let Some(auth) = auth_header {
        let expected_auth = format!("Bearer {}", state._config.api_key);
        if auth != expected_auth {
            return Ok(create_error_response(
                StatusCode::UNAUTHORIZED,
                1001,
                "Invalid API key",
            ));
        }
    } else {
        return Ok(create_error_response(
            StatusCode::UNAUTHORIZED,
            1001,
            "Missing authorization header",
        ));
    }

    // Route to appropriate handler
    let response = match (method, path) {
        ("POST", "/api/paas/v4/chat/completions") => handle_chat_completion(req, &state).await,
        ("POST", "/api/paas/v4/embeddings") => handle_embeddings(req, &state).await,
        ("GET", _) if path.starts_with("/api/paas/v4/files/") => {
            handle_file_retrieval(path, &state).await
        },
        _ => Ok(create_error_response(
            StatusCode::NOT_FOUND,
            0,
            "Endpoint not found",
        )),
    };

    response
}

/// Handle chat completion requests
#[allow(dead_code)]
async fn handle_chat_completion(
    req: Request<Body>,
    state: &MockServerState,
) -> Result<Response<Body>, hyper::Error> {
    let body = hyper::body::to_bytes(req.into_body()).await.unwrap();
    let _request_body: serde_json::Value = serde_json::from_slice(&body).unwrap_or(json!({}));

    let response_body =
        if let Some(custom_response) = state.get_response("/api/paas/v4/chat/completions") {
            custom_response
        } else {
            json!({
                "id": "chatcmpl-1234567890",
                "object": "chat.completion",
                "created": 1704067200,
                "model": "glm-4",
                "choices": [{
                    "index": 0,
                    "message": {
                        "role": "assistant",
                        "content": "This is a mock response from the integration test server."
                    },
                    "finish_reason": "stop"
                }],
                "usage": {
                    "prompt_tokens": 10,
                    "completion_tokens": 20,
                    "total_tokens": 30
                }
            })
        };

    Ok(Response::new(Body::from(
        serde_json::to_string(&response_body).unwrap(),
    )))
}

/// Handle embedding requests
#[allow(dead_code)]
async fn handle_embeddings(
    req: Request<Body>,
    state: &MockServerState,
) -> Result<Response<Body>, hyper::Error> {
    let _body = hyper::body::to_bytes(req.into_body()).await.unwrap();

    let response_body = if let Some(custom_response) = state.get_response("/api/paas/v4/embeddings")
    {
        custom_response
    } else {
        json!({
            "object": "list",
            "data": [{
                "object": "embedding",
                "embedding": [0.002, -0.002, 0.004, 0.001, -0.003, 0.002],
                "index": 0
            }],
            "model": "embedding-2",
            "usage": {
                "prompt_tokens": 8,
                "total_tokens": 8
            }
        })
    };

    Ok(Response::new(Body::from(
        serde_json::to_string(&response_body).unwrap(),
    )))
}

/// Handle file retrieval requests
#[allow(dead_code)]
async fn handle_file_retrieval(
    path: &str,
    state: &MockServerState,
) -> Result<Response<Body>, hyper::Error> {
    let response_body = if let Some(custom_response) = state.get_response(path) {
        custom_response
    } else {
        json!({
            "id": "file-1234567890",
            "object": "file",
            "bytes": 1024,
            "created_at": 1704067200,
            "filename": "test.txt",
            "purpose": "assistants"
        })
    };

    Ok(Response::new(Body::from(
        serde_json::to_string(&response_body).unwrap(),
    )))
}

/// Create an error response
#[allow(dead_code)]
fn create_error_response(status: StatusCode, code: u16, message: &str) -> Response<Body> {
    let error_body = json!({
        "error": {
            "code": code,
            "message": message
        }
    });

    let mut response = Response::new(Body::from(serde_json::to_string(&error_body).unwrap()));
    *response.status_mut() = status;
    response
}

/// Mock server client for testing
#[derive(Debug, Clone)]
pub struct MockServerClient {
    base_url: String,
}

impl MockServerClient {
    /// Create a new mock server client
    pub fn new(base_url: String) -> Self {
        Self { base_url }
    }

    /// Get the base URL
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Construct a full URL for an endpoint
    pub fn url(&self, endpoint: &str) -> String {
        format!("{}{}", self.base_url, endpoint)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_server_config_default() {
        let config = MockServerConfig::default();
        assert_eq!(config.api_key, "test.12345678901234567890");
        assert_eq!(config.base_url, "http://127.0.0.1:9876");
    }

    #[test]
    fn test_mock_server_state_register_response() {
        let config = MockServerConfig::default();
        let state = MockServerState::new(config);

        let response = json!({"test": "data"});
        state.register_response("/test", response.clone());

        assert_eq!(state.get_response("/test"), Some(response));
    }

    #[test]
    fn test_mock_server_client_url() {
        let client = MockServerClient::new("http://127.0.0.1:9876".to_string());
        assert_eq!(client.base_url(), "http://127.0.0.1:9876");
        assert_eq!(client.url("/api/test"), "http://127.0.0.1:9876/api/test");
    }
}
