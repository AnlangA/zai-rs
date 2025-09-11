//! # HTTP Client Implementation
//!
//! Provides a robust HTTP client for communicating with the Zhipu AI API.
//! This module implements connection pooling, error handling, and request/response processing.
//!
//! ## Features
//!
//! - **Connection Pooling** - Reuses HTTP connections for better performance
//! - **Error Handling** - Comprehensive error parsing and reporting
//! - **Authentication** - Bearer token authentication support
//! - **Logging** - Detailed request/response logging for debugging
//!
//! ## Usage
//!
//! The `HttpClient` trait provides a standardized interface for making HTTP requests
//! to the Zhipu AI API endpoints.

use log::{debug, info};
use serde::Deserialize;
use std::sync::OnceLock;

#[derive(Debug, Deserialize)]
struct ApiErrorEnvelope {
    error: ApiError,
}

#[derive(Debug, Deserialize)]
struct ApiError {
    code: ErrorCode,
    message: String,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum ErrorCode {
    Str(String),
    Num(i64),
}

impl std::fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorCode::Str(s) => write!(f, "{}", s),
            ErrorCode::Num(n) => write!(f, "{}", n),
        }
    }
}

/// A single shared HTTP client for connection pooling and TLS reuse.
///
/// This static instance ensures that all HTTP requests use the same underlying
/// connection pool, improving performance by reusing TCP connections and TLS sessions.
static HTTP_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

/// Gets the shared HTTP client instance.
///
/// Initializes the client on first call with default configuration.
/// Subsequent calls return the same instance for connection reuse.
fn http_client() -> &'static reqwest::Client {
    HTTP_CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .build()
            .expect("Failed to build reqwest Client")
    })
}

/// Trait for HTTP clients that communicate with the Zhipu AI API.
///
/// This trait provides a standardized interface for making HTTP requests
/// to Zhipu AI API endpoints with proper authentication and error handling.
///
/// ## Type Parameters
///
/// - `Body` - The request body type that implements `Serialize`
/// - `ApiUrl` - The API URL type that can be referenced as a string
/// - `ApiKey` - The API key type that can be referenced as a string
pub trait HttpClient {
    /// The request body type that must implement JSON serialization.
    type Body: serde::Serialize;

    /// The API URL type that must be convertible to a string reference.
    type ApiUrl: AsRef<str>;

    /// The API key type that must be convertible to a string reference.
    type ApiKey: AsRef<str>;

    /// Returns a reference to the API URL.
    fn api_url(&self) -> &Self::ApiUrl;

    /// Returns a reference to the API key for authentication.
    fn api_key(&self) -> &Self::ApiKey;

    /// Returns a reference to the request body.
    fn body(&self) -> &Self::Body;

    /// Sends a POST request to the API endpoint.
    ///
    /// This method handles:
    /// - JSON serialization of the request body
    /// - Bearer token authentication
    /// - Error response parsing and reporting
    /// - Connection reuse through the shared HTTP client
    ///
    /// Returns the HTTP response on success, or an error on failure.
    fn post(&self) -> impl std::future::Future<Output = anyhow::Result<reqwest::Response>> + Send {
        let body_compact = serde_json::to_string(self.body());
        // Only compute pretty JSON when info-level logging is enabled to avoid extra serialization cost
        let body_pretty_opt = if log::log_enabled!(log::Level::Info) {
            Some(serde_json::to_string_pretty(self.body()).unwrap_or_default())
        } else {
            None
        };
        let url = self.api_url().as_ref().to_owned();
        let key = self.api_key().as_ref().to_owned();
        async move {
            let body = body_compact?;
            if let Some(pretty) = body_pretty_opt {
                info!("Request body: {}", pretty);
            }
            let resp = http_client()
                .post(url)
                .bearer_auth(key)
                .header("Content-Type", "application/json")
                .body(body)
                .send()
                .await?;

            let status = resp.status();
            if status.is_success() {
                return Ok(resp);
            }
            // Debug headers for troubleshooting on non-2xx
            debug!(
                "HTTP {} {} headers: {:?}",
                status.as_u16(),
                status.canonical_reason().unwrap_or(""),
                resp.headers()
            );

            // Non-success HTTP status: parse error JSON and return Err
            let text = resp.text().await.unwrap_or_default();
            if let Ok(parsed) = serde_json::from_str::<ApiErrorEnvelope>(&text) {
                let code_str = parsed.error.code.to_string();
                return Err(anyhow::anyhow!(
                    "HTTP {} {} | code={} | message={}",
                    status.as_u16(),
                    status.canonical_reason().unwrap_or(""),
                    code_str,
                    parsed.error.message
                ));
            } else {
                return Err(anyhow::anyhow!(
                    "HTTP {} {} | body={}",
                    status.as_u16(),
                    status.canonical_reason().unwrap_or(""),
                    text
                ));
            }
        }
    }

    /// Sends a GET request to the API endpoint.
    ///
    /// This method is used for endpoints that don't require a request body.
    /// It handles authentication and error response parsing similar to `post()`.
    ///
    /// Returns the HTTP response on success, or an error on failure.
    fn get(&self) -> impl std::future::Future<Output = anyhow::Result<reqwest::Response>> + Send {
        let url = self.api_url().as_ref().to_owned();
        let key = self.api_key().as_ref().to_owned();
        async move {
            let resp = http_client().get(url).bearer_auth(key).send().await?;

            let status = resp.status();
            if status.is_success() {
                return Ok(resp);
            }
            // Debug headers for troubleshooting on non-2xx
            debug!(
                "HTTP {} {} headers: {:?}",
                status.as_u16(),
                status.canonical_reason().unwrap_or(""),
                resp.headers()
            );

            // Non-success HTTP status: parse error JSON and return Err
            let text = resp.text().await.unwrap_or_default();
            if let Ok(parsed) = serde_json::from_str::<ApiErrorEnvelope>(&text) {
                let code_str = parsed.error.code.to_string();
                return Err(anyhow::anyhow!(
                    "HTTP {} {} | code={} | message={}",
                    status.as_u16(),
                    status.canonical_reason().unwrap_or(""),
                    code_str,
                    parsed.error.message
                ));
            } else {
                return Err(anyhow::anyhow!(
                    "HTTP {} {} | body={}",
                    status.as_u16(),
                    status.canonical_reason().unwrap_or(""),
                    text
                ));
            }
        }
    }
}
