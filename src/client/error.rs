//! Custom error types for ZAI-RS
//!
//! This module defines comprehensive error types that map to the ZhipuAI API error codes
//! as documented at https://docs.bigmodel.cn/cn/api/api-code

use thiserror::Error;

/// Main error type for the ZAI-RS SDK
#[derive(Error, Debug)]
pub enum ZaiError {
    /// HTTP status errors
    #[error("HTTP error [{status}]: {message}")]
    HttpError { status: u16, message: String },

    /// Authentication and authorization errors
    #[error("Authentication error [{code}]: {message}")]
    AuthError { code: u16, message: String },

    /// Account-related errors
    #[error("Account error [{code}]: {message}")]
    AccountError { code: u16, message: String },

    /// API call errors
    #[error("API error [{code}]: {message}")]
    ApiError { code: u16, message: String },

    /// Rate limiting and quota errors
    #[error("Rate limit error [{code}]: {message}")]
    RateLimitError { code: u16, message: String },

    /// Content policy errors
    #[error("Content policy error [{code}]: {message}")]
    ContentPolicyError { code: u16, message: String },

    /// File processing errors
    #[error("File error [{code}]: {message}")]
    FileError { code: u16, message: String },

    /// Network/IO errors
    #[error("Network error: {0}")]
    NetworkError(reqwest::Error),

    /// JSON parsing errors
    #[error("JSON error: {0}")]
    JsonError(serde_json::Error),

    /// Other errors
    #[error("Unknown error [{code}]: {message}")]
    Unknown { code: u16, message: String },
}

impl ZaiError {
    /// Convert an HTTP status code and API error response to a ZaiError
    pub fn from_api_response(status: u16, api_code: u16, api_message: String) -> Self {
        match (status, api_code) {
            // HTTP Status Errors
            (400, _) => ZaiError::HttpError {
                status,
                message: if api_message.is_empty() {
                    "Bad request - check your parameters".to_string()
                } else {
                    api_message
                },
            },
            (401, _) => ZaiError::HttpError {
                status,
                message: "Unauthorized - check your API key".to_string(),
            },
            (404, _) => ZaiError::HttpError {
                status,
                message: "Not found - the requested resource doesn't exist".to_string(),
            },
            (429, _) => ZaiError::HttpError {
                status,
                message: if api_message.is_empty() {
                    "Too many requests - rate limit exceeded".to_string()
                } else {
                    api_message
                },
            },
            (434, _) => ZaiError::HttpError {
                status,
                message: "No API permission - feature not available".to_string(),
            },
            (435, _) => ZaiError::HttpError {
                status,
                message: "File size exceeds 100MB limit".to_string(),
            },
            (500, _) => ZaiError::HttpError {
                status,
                message: "Internal server error - try again later".to_string(),
            },

            // Business Error Codes
            (_, 500) => ZaiError::Unknown {
                code: api_code,
                message: "Internal server error".to_string(),
            },

            // Authentication errors (1000-1004, 1100)
            (1000..=1004, _) | (_, 1100) => ZaiError::AuthError {
                code: api_code,
                message: api_message,
            },

            // Account errors (1110-1121)
            (1110..=1121, _) => ZaiError::AccountError {
                code: api_code,
                message: api_message,
            },

            // API call errors (1200-1234)
            (1200..=1234, _) => ZaiError::ApiError {
                code: api_code,
                message: api_message,
            },

            // Rate limiting errors (1300-1309)
            (1300..=1309, _) => ZaiError::RateLimitError {
                code: api_code,
                message: api_message,
            },

            // Default to unknown error
            _ => ZaiError::Unknown {
                code: api_code,
                message: api_message,
            },
        }
    }

    /// Check if the error is a rate limit error
    pub fn is_rate_limit(&self) -> bool {
        matches!(self, ZaiError::RateLimitError { .. })
    }

    /// Check if the error is an authentication error
    pub fn is_auth_error(&self) -> bool {
        matches!(self, ZaiError::AuthError { .. })
    }

    /// Check if the error is a client error (4xx)
    pub fn is_client_error(&self) -> bool {
        match self {
            ZaiError::HttpError { status, .. } => *status >= 400 && *status < 500,
            ZaiError::AuthError { .. }
            | ZaiError::AccountError { .. }
            | ZaiError::ApiError { .. }
            | ZaiError::RateLimitError { .. }
            | ZaiError::ContentPolicyError { .. }
            | ZaiError::FileError { .. } => true,
            _ => false,
        }
    }

    /// Check if the error is a server error (5xx)
    pub fn is_server_error(&self) -> bool {
        match self {
            ZaiError::HttpError { status, .. } => *status >= 500,
            ZaiError::Unknown { code, .. } => *code >= 500,
            _ => false,
        }
    }
    /// Get a compact representation of error suitable for logging
    pub fn compact(&self) -> String {
        match self {
            ZaiError::HttpError { status, message } => {
                format!("HTTP[{}]: {}", status, message)
            }
            ZaiError::AuthError { code, message } => {
                format!("AUTH[{}]: {}", code, message)
            }
            ZaiError::AccountError { code, message } => {
                format!("ACCOUNT[{}]: {}", code, message)
            }
            ZaiError::ApiError { code, message } => {
                format!("API[{}]: {}", code, message)
            }
            ZaiError::RateLimitError { code, message } => {
                format!("RATE_LIMIT[{}]: {}", code, message)
            }
            ZaiError::ContentPolicyError { code, message } => {
                format!("POLICY[{}]: {}", code, message)
            }
            ZaiError::FileError { code, message } => {
                format!("FILE[{}]: {}", code, message)
            }
            ZaiError::NetworkError(err) => {
                format!("NETWORK: {}", err)
            }
            ZaiError::JsonError(err) => {
                format!("JSON: {}", err)
            }
            ZaiError::Unknown { code, message } => {
                format!("UNKNOWN[{}]: {}", code, message)
            }
        }
    }

    /// Get error code if available
    pub fn code(&self) -> Option<u16> {
        match self {
            ZaiError::HttpError { status, .. } => Some(*status),
            ZaiError::AuthError { code, .. } => Some(*code),
            ZaiError::AccountError { code, .. } => Some(*code),
            ZaiError::ApiError { code, .. } => Some(*code),
            ZaiError::RateLimitError { code, .. } => Some(*code),
            ZaiError::ContentPolicyError { code, .. } => Some(*code),
            ZaiError::FileError { code, .. } => Some(*code),
            ZaiError::NetworkError(_) => None,
            ZaiError::JsonError(_) => None,
            ZaiError::Unknown { code, .. } => Some(*code),
        }
    }

    /// Get error message
    pub fn message(&self) -> String {
        match self {
            ZaiError::HttpError { message, .. } => message.clone(),
            ZaiError::AuthError { message, .. } => message.clone(),
            ZaiError::AccountError { message, .. } => message.clone(),
            ZaiError::ApiError { message, .. } => message.clone(),
            ZaiError::RateLimitError { message, .. } => message.clone(),
            ZaiError::ContentPolicyError { message, .. } => message.clone(),
            ZaiError::FileError { message, .. } => message.clone(),
            ZaiError::NetworkError(err) => err.to_string(),
            ZaiError::JsonError(err) => err.to_string(),
            ZaiError::Unknown { message, .. } => message.clone(),
        }
    }
}

/// Type alias for Result with ZaiError
pub type ZaiResult<T> = Result<T, ZaiError>;

/// Convert from reqwest::Error to ZaiError
impl From<reqwest::Error> for ZaiError {
    fn from(err: reqwest::Error) -> Self {
        if let Some(status) = err.status() {
            ZaiError::from_api_response(status.as_u16(), 0, err.to_string())
        } else {
            ZaiError::NetworkError(err)
        }
    }
}

/// Convert from serde_json::Error to ZaiError
impl From<serde_json::Error> for ZaiError {
    fn from(err: serde_json::Error) -> Self {
        ZaiError::JsonError(err)
    }
}

/// Convert from validator::ValidationErrors to ZaiError
impl From<validator::ValidationErrors> for ZaiError {
    fn from(err: validator::ValidationErrors) -> Self {
        ZaiError::ApiError {
            code: 1200,
            message: format!("Validation error: {:?}", err),
        }
    }
}

/// Convert from std::io::Error to ZaiError
impl From<std::io::Error> for ZaiError {
    fn from(err: std::io::Error) -> Self {
        ZaiError::Unknown {
            code: 0,
            message: err.to_string(),
        }
    }
}
