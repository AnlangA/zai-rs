//! Error types and handling for the web chat application

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;

/// Main application error type
#[derive(Debug, Error)]
pub enum AppError {
    #[error("Configuration error: {0}")]
    Config(#[from] crate::server::config::ConfigError),

    #[error("Chat API error: {0}")]
    ChatApi(String),

    #[error("Session not found: {0}")]
    SessionNotFound(String),

    #[error("Session expired: {0}")]
    SessionExpired(String),

    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    #[error("Streaming error: {0}")]
    StreamingError(String),

    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    #[error("Internal server error")]
    InternalError,

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Service unavailable")]
    ServiceUnavailable,
}

/// Error response structure for API responses
#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: ErrorDetail,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorDetail {
    pub code: String,
    pub message: String,
    pub details: Option<serde_json::Value>,
    pub timestamp: String,
}

impl AppError {
    /// Get the HTTP status code for this error
    pub fn status_code(&self) -> StatusCode {
        match self {
            AppError::Config(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::ChatApi(_) => StatusCode::BAD_GATEWAY,
            AppError::SessionNotFound(_) => StatusCode::NOT_FOUND,
            AppError::SessionExpired(_) => StatusCode::GONE,
            AppError::InvalidRequest(_) => StatusCode::BAD_REQUEST,
            AppError::StreamingError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::RateLimitExceeded => StatusCode::TOO_MANY_REQUESTS,
            AppError::InternalError => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::BadRequest(_) => StatusCode::BAD_REQUEST,
            AppError::Unauthorized => StatusCode::UNAUTHORIZED,
            AppError::ServiceUnavailable => StatusCode::SERVICE_UNAVAILABLE,
        }
    }

    /// Get the error code for this error
    pub fn error_code(&self) -> &'static str {
        match self {
            AppError::Config(_) => "CONFIG_ERROR",
            AppError::ChatApi(_) => "CHAT_API_ERROR",
            AppError::SessionNotFound(_) => "SESSION_NOT_FOUND",
            AppError::SessionExpired(_) => "SESSION_EXPIRED",
            AppError::InvalidRequest(_) => "INVALID_REQUEST",
            AppError::StreamingError(_) => "STREAMING_ERROR",
            AppError::RateLimitExceeded => "RATE_LIMIT_EXCEEDED",
            AppError::InternalError => "INTERNAL_ERROR",
            AppError::BadRequest(_) => "BAD_REQUEST",
            AppError::Unauthorized => "UNAUTHORIZED",
            AppError::ServiceUnavailable => "SERVICE_UNAVAILABLE",
        }
    }

    /// Create an error response
    pub fn to_error_response(&self) -> ErrorResponse {
        ErrorResponse {
            error: ErrorDetail {
                code: self.error_code().to_string(),
                message: self.to_string(),
                details: None,
                timestamp: chrono::Utc::now().to_rfc3339(),
            },
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let error_response = self.to_error_response();

        tracing::error!(
            error.code = %error_response.error.code,
            error.message = %error_response.error.message,
            error.status = %status.as_u16(),
            "Application error occurred"
        );

        (status, Json(error_response)).into_response()
    }
}

/// Result type alias for application operations
pub type AppResult<T> = Result<T, AppError>;

/// Convert from local ClientError to AppError
impl From<crate::client::error_handler::ClientError> for AppError {
    fn from(error: crate::client::error_handler::ClientError) -> Self {
        AppError::ChatApi(error.to_string())
    }
}

/// Convert from validation errors to AppError
impl From<validator::ValidationErrors> for AppError {
    fn from(errors: validator::ValidationErrors) -> Self {
        AppError::InvalidRequest(errors.to_string())
    }
}

/// Convert from JSON serialization errors to AppError
impl From<serde_json::Error> for AppError {
    fn from(error: serde_json::Error) -> Self {
        AppError::InvalidRequest(format!("JSON error: {}", error))
    }
}

/// Convert from UUID parsing errors to AppError
impl From<uuid::Error> for AppError {
    fn from(error: uuid::Error) -> Self {
        AppError::InvalidRequest(format!("Invalid UUID: {}", error))
    }
}

/// Convert from std::time::SystemTimeError to AppError
impl From<std::time::SystemTimeError> for AppError {
    fn from(error: std::time::SystemTimeError) -> Self {
        AppError::InternalError
    }
}

/// Error handling utilities
pub mod error_utils {
    use super::*;

    /// Create a bad request error with a custom message
    pub fn bad_request(message: impl Into<String>) -> AppError {
        AppError::BadRequest(message.into())
    }

    /// Create an invalid request error with a custom message
    pub fn invalid_request(message: impl Into<String>) -> AppError {
        AppError::InvalidRequest(message.into())
    }

    /// Create a session not found error
    pub fn session_not_found(session_id: impl Into<String>) -> AppError {
        AppError::SessionNotFound(session_id.into())
    }

    /// Create a streaming error with a custom message
    pub fn streaming_error(message: impl Into<String>) -> AppError {
        AppError::StreamingError(message.into())
    }
}
