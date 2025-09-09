//! Structured error handling for API responses

use crate::model::chat_base_response::ErrorResponse;
use log::{error, warn};
use reqwest::Response;
use serde_json::from_str;

/// Enhanced error information with context
#[derive(Debug)]
pub struct ApiErrorInfo {
    pub status_code: u16,
    pub error_code: String,
    pub message: String,
    pub raw_response: String,
}

impl std::fmt::Display for ApiErrorInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "API Error [{}]: {} - {}",
            self.status_code, self.error_code, self.message
        )
    }
}

impl std::error::Error for ApiErrorInfo {}

impl From<serde_json::Error> for ApiErrorInfo {
    fn from(e: serde_json::Error) -> Self {
        ApiErrorInfo {
            status_code: 0,
            error_code: "serialization_error".to_string(),
            message: format!("JSON serialization error: {}", e),
            raw_response: String::new(),
        }
    }
}

impl From<anyhow::Error> for ApiErrorInfo {
    fn from(e: anyhow::Error) -> Self {
        ApiErrorInfo {
            status_code: 0,
            error_code: "internal_error".to_string(),
            message: format!("Internal error: {}", e),
            raw_response: String::new(),
        }
    }
}

/// Handle API error responses with structured parsing
pub async fn handle_api_error(resp: Response) -> ApiErrorInfo {
    let status = resp.status();
    let status_code = status.as_u16();
    let text = resp
        .text()
        .await
        .unwrap_or_else(|_| "Failed to get response text".to_string());

    // Try to parse as structured error
    if let Ok(error_resp) = from_str::<ErrorResponse>(&text) {
        let error_info = ApiErrorInfo {
            status_code,
            error_code: error_resp.error.code.clone(),
            message: error_resp.error.message.clone(),
            raw_response: text,
        };

        // Log with appropriate level based on error type
        match status_code {
            400..=499 => {
                if error_resp.is_validation_error() {
                    warn!("Validation error: {}", error_info);
                } else {
                    error!("Client error: {}", error_info);
                }
            }
            500..=599 => error!("Server error: {}", error_info),
            _ => error!("Unknown API error: {}", error_info),
        }

        error_info
    } else {
        // Fallback for unstructured errors
        let error_info = ApiErrorInfo {
            status_code,
            error_code: "unknown".to_string(),
            message: format!("Unstructured error response: {}", text),
            raw_response: text,
        };

        error!("Unstructured API error: {}", error_info);
        error_info
    }
}

/// Enhanced result type for API responses
pub type ApiResult<T> = Result<T, ApiErrorInfo>;

/// Convert reqwest errors to our error type
pub fn convert_reqwest_error(e: reqwest::Error) -> ApiErrorInfo {
    ApiErrorInfo {
        status_code: e.status().map_or(0, |s| s.as_u16()),
        error_code: "network_error".to_string(),
        message: e.to_string(),
        raw_response: String::new(),
    }
}

/// Helper function to check if error is retryable
pub fn is_retryable_error(error_info: &ApiErrorInfo) -> bool {
    match error_info.status_code {
        429 => true,       // Rate limit
        500..=599 => true, // Server errors
        _ => false,
    }
}
