//! # HTTP Client Implementation

//!

//! Provides a robust HTTP client for communicating with the Zhipu AI API.

//! This module implements connection pooling, error handling, and request/response processing.

//!

//! ## Features

//!

//! - Connection Pooling - Reuses HTTP connections for better performance

//! - Error Handling - Comprehensive error parsing and reporting

//! - Authentication - Bearer token authentication support

//! - Logging - Detailed request/response logging for debugging

//!

//! ## Usage

//!

//! The `HttpClient` trait provides a standardized interface for making HTTP requests

//! to the Zhipu AI API endpoints.

use log::{debug, info};

use serde::Deserialize;

use std::sync::OnceLock;

use crate::client::error::{ZaiError, ZaiResult};

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

fn to_api_code(code: &ErrorCode) -> u16 {
    match code {
        ErrorCode::Num(n) => (*n).try_into().unwrap_or(0),
        ErrorCode::Str(s) => s.parse::<u16>().unwrap_or(0),
    }
}

/// A single shared HTTP client for connection pooling and TLS reuse.

static HTTP_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

/// Gets the shared HTTP client instance.

fn http_client() -> &'static reqwest::Client {
    HTTP_CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .build()
            .expect("Failed to build reqwest Client")
    })
}

/// Trait for HTTP clients that communicate with the Zhipu AI API.

pub trait HttpClient {
    type Body: serde::Serialize;
    type ApiUrl: AsRef<str>;
    type ApiKey: AsRef<str>;

    fn api_url(&self) -> &Self::ApiUrl;
    fn api_key(&self) -> &Self::ApiKey;
    fn body(&self) -> &Self::Body;

    /// Sends a POST request to the API endpoint.

    fn post(&self) -> impl std::future::Future<Output = ZaiResult<reqwest::Response>> + Send {
        let body_compact = serde_json::to_string(self.body());

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

            // Non-success HTTP status: parse error JSON and return ZaiError

            let text = resp.text().await.unwrap_or_default();

            if let Ok(parsed) = serde_json::from_str::<ApiErrorEnvelope>(&text) {
                let api_code = to_api_code(&parsed.error.code);
                return Err(ZaiError::from_api_response(
                    status.as_u16(),
                    api_code,
                    parsed.error.message,
                ));
            } else {
                return Err(ZaiError::from_api_response(status.as_u16(), 0, text));
            }
        }
    }

    /// Sends a GET request to the API endpoint.

    fn get(&self) -> impl std::future::Future<Output = ZaiResult<reqwest::Response>> + Send {
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

            // Non-success HTTP status: parse error JSON and return ZaiError

            let text = resp.text().await.unwrap_or_default();

            if let Ok(parsed) = serde_json::from_str::<ApiErrorEnvelope>(&text) {
                let api_code = to_api_code(&parsed.error.code);
                return Err(ZaiError::from_api_response(
                    status.as_u16(),
                    api_code,
                    parsed.error.message,
                ));
            } else {
                return Err(ZaiError::from_api_response(status.as_u16(), 0, text));
            }
        }
    }
}
