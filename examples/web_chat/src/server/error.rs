//! HTTP-safe error mapping for the example API.

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;

/// Errors that can reach an HTTP handler.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("upstream API request failed: {0}")]
    Upstream(#[from] zai_rs::ZaiError),
    #[error("upstream response did not contain assistant text")]
    InvalidUpstreamResponse,
    #[error("session not found: {0}")]
    SessionNotFound(String),
    #[error("session expired: {0}")]
    SessionExpired(String),
    #[error("invalid request: {0}")]
    InvalidRequest(String),
    #[error("request body is not valid JSON")]
    InvalidJson,
    #[error("request Content-Type is not application/json")]
    UnsupportedMediaType,
    #[error("request body is too large")]
    PayloadTooLarge,
    #[error("rate limit exceeded")]
    RateLimitExceeded,
}

impl AppError {
    pub(crate) fn status_and_code(&self) -> (StatusCode, &'static str) {
        match self {
            Self::Upstream(_) | Self::InvalidUpstreamResponse => {
                (StatusCode::BAD_GATEWAY, "UPSTREAM_ERROR")
            },
            Self::SessionNotFound(_) => (StatusCode::NOT_FOUND, "SESSION_NOT_FOUND"),
            Self::SessionExpired(_) => (StatusCode::GONE, "SESSION_EXPIRED"),
            Self::InvalidRequest(_) => (StatusCode::BAD_REQUEST, "INVALID_REQUEST"),
            Self::InvalidJson => (StatusCode::BAD_REQUEST, "INVALID_JSON"),
            Self::UnsupportedMediaType => {
                (StatusCode::UNSUPPORTED_MEDIA_TYPE, "UNSUPPORTED_MEDIA_TYPE")
            },
            Self::PayloadTooLarge => (StatusCode::PAYLOAD_TOO_LARGE, "PAYLOAD_TOO_LARGE"),
            Self::RateLimitExceeded => (StatusCode::TOO_MANY_REQUESTS, "RATE_LIMIT_EXCEEDED"),
        }
    }

    fn public_message(&self) -> String {
        match self {
            Self::Upstream(_) | Self::InvalidUpstreamResponse => {
                "The upstream service could not complete the request.".to_owned()
            },
            Self::SessionNotFound(_) => "The requested session was not found.".to_owned(),
            Self::SessionExpired(_) => "The requested session has expired.".to_owned(),
            Self::InvalidRequest(message) => message.clone(),
            Self::InvalidJson => "Request body must be valid JSON.".to_owned(),
            Self::UnsupportedMediaType => "Content-Type must be application/json.".to_owned(),
            Self::PayloadTooLarge => "Request body exceeds the server limit.".to_owned(),
            Self::RateLimitExceeded => "Too many requests. Please try again later.".to_owned(),
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code) = self.status_and_code();
        if let Self::Upstream(error) = &self {
            tracing::warn!(%status, %code, upstream_code = ?error.code(), "request failed");
        } else {
            tracing::warn!(%status, %code, "request failed");
        }
        let message = self.public_message();
        (
            status,
            Json(ErrorResponse {
                error: ErrorDetail { code, message },
            }),
        )
            .into_response()
    }
}

impl From<validator::ValidationErrors> for AppError {
    fn from(_error: validator::ValidationErrors) -> Self {
        // Validator diagnostics are useful to developers but are not a stable
        // public protocol and can reveal internal field rules. Keep the HTTP
        // response intentionally generic.
        Self::InvalidRequest("Request validation failed.".to_owned())
    }
}

#[derive(Serialize)]
struct ErrorResponse {
    error: ErrorDetail,
}

#[derive(Serialize)]
struct ErrorDetail {
    code: &'static str,
    message: String,
}

pub type AppResult<T> = Result<T, AppError>;
