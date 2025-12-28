//! Custom error types for ZAI-RS
//!
//! This module defines comprehensive error types that map to the ZhipuAI API error codes
//! as documented at https://docs.bigmodel.cn/cn/api/api-code
//!
//! The error handling supports both HTTP and WebSocket connections with appropriate
//! error mapping for each protocol.

use thiserror::Error;

/// Main error type for the ZAI-RS SDK
#[derive(Error, Debug)]
pub enum ZaiError {
    /// HTTP status errors
    #[error("HTTP error [{status}]: {message}")]
    HttpError { status: u16, message: String },

    /// WebSocket connection errors
    #[error("WebSocket error [{code}]: {message}")]
    WebSocketError { code: u16, message: String },

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

    /// Network/IO errors (includes HTTP and WebSocket transport errors)
    #[error("Network error: {0}")]
    NetworkError(String),

    /// Connection errors (for WebSocket connection establishment)
    #[error("Connection error: {0}")]
    ConnectionError(String),

    /// Protocol errors (WebSocket protocol violations)
    #[error("Protocol error: {0}")]
    ProtocolError(String),

    /// JSON parsing errors
    #[error("JSON error: {0}")]
    JsonError(serde_json::Error),

    /// TLS/SSL errors
    #[error("TLS error: {0}")]
    TlsError(String),

    /// Timeout errors
    #[error("Timeout error: {0}")]
    TimeoutError(String),

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

    /// Create an error from HTTP status code (used by WebSocket client)
    pub fn from_status_code(status: u16, message: Option<String>) -> Self {
        let msg = message.unwrap_or_else(|| match status {
            400 => "Bad request".to_string(),
            401 => "Unauthorized".to_string(),
            403 => "Forbidden".to_string(),
            404 => "Not found".to_string(),
            429 => "Too many requests".to_string(),
            500 => "Internal server error".to_string(),
            502 => "Bad gateway".to_string(),
            503 => "Service unavailable".to_string(),
            504 => "Gateway timeout".to_string(),
            _ => format!("HTTP status {}", status),
        });

        ZaiError::HttpError {
            status,
            message: msg,
        }
    }

    /// Create a WebSocket-specific error
    pub fn websocket_error(code: u16, message: impl Into<String>) -> Self {
        ZaiError::WebSocketError {
            code,
            message: message.into(),
        }
    }

    /// Create a WebSocket connection error
    pub fn websocket_connection_error(message: impl Into<String>) -> Self {
        ZaiError::ConnectionError(message.into())
    }

    /// Create a WebSocket protocol error
    pub fn websocket_protocol_error(message: impl Into<String>) -> Self {
        ZaiError::ProtocolError(message.into())
    }

    /// Create a timeout error
    pub fn timeout(message: impl Into<String>) -> Self {
        ZaiError::TimeoutError(message.into())
    }

    /// Create a TLS error
    pub fn tls(message: impl Into<String>) -> Self {
        ZaiError::TlsError(message.into())
    }

    /// Check if the error is a rate limit error
    pub fn is_rate_limit(&self) -> bool {
        matches!(self, ZaiError::RateLimitError { .. })
    }

    /// Check if the error is an authentication error
    pub fn is_auth_error(&self) -> bool {
        matches!(self, ZaiError::AuthError { .. })
    }

    /// Check if the error is a network/connection error
    pub fn is_network_error(&self) -> bool {
        matches!(
            self,
            ZaiError::NetworkError(_)
                | ZaiError::ConnectionError(_)
                | ZaiError::TlsError(_)
                | ZaiError::TimeoutError(_)
        )
    }

    /// Check if the error is a WebSocket-specific error
    pub fn is_websocket_error(&self) -> bool {
        matches!(
            self,
            ZaiError::WebSocketError { .. }
                | ZaiError::ConnectionError(_)
                | ZaiError::ProtocolError(_)
        )
    }

    /// Check if the error is retryable (transient)
    pub fn is_retryable(&self) -> bool {
        match self {
            ZaiError::RateLimitError { .. } => true,
            ZaiError::NetworkError(_) => true,
            ZaiError::ConnectionError(_) => true,
            ZaiError::TimeoutError(_) => true,
            ZaiError::TlsError(_) => false, // TLS errors are usually not retryable immediately
            ZaiError::HttpError { status, .. } => matches!(status, 429 | 500 | 502 | 503 | 504),
            ZaiError::Unknown { code, .. } => matches!(code, 500 | 502 | 503 | 504),
            _ => false,
        }
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
            ZaiError::WebSocketError { code, .. } => *code >= 400 && *code < 500,
            _ => false,
        }
    }

    /// Check if the error is a server error (5xx)
    pub fn is_server_error(&self) -> bool {
        match self {
            ZaiError::HttpError { status, .. } => *status >= 500,
            ZaiError::WebSocketError { code, .. } => *code >= 500,
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
            ZaiError::WebSocketError { code, message } => {
                format!("WS[{}]: {}", code, message)
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
            ZaiError::ConnectionError(err) => {
                format!("CONN: {}", err)
            }
            ZaiError::ProtocolError(err) => {
                format!("PROTO: {}", err)
            }
            ZaiError::JsonError(err) => {
                format!("JSON: {}", err)
            }
            ZaiError::TlsError(err) => {
                format!("TLS: {}", err)
            }
            ZaiError::TimeoutError(err) => {
                format!("TIMEOUT: {}", err)
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
            ZaiError::WebSocketError { code, .. } => Some(*code),
            ZaiError::AuthError { code, .. } => Some(*code),
            ZaiError::AccountError { code, .. } => Some(*code),
            ZaiError::ApiError { code, .. } => Some(*code),
            ZaiError::RateLimitError { code, .. } => Some(*code),
            ZaiError::ContentPolicyError { code, .. } => Some(*code),
            ZaiError::FileError { code, .. } => Some(*code),
            ZaiError::NetworkError(_)
            | ZaiError::ConnectionError(_)
            | ZaiError::ProtocolError(_)
            | ZaiError::TlsError(_)
            | ZaiError::TimeoutError(_) => None,
            ZaiError::JsonError(_) => None,
            ZaiError::Unknown { code, .. } => Some(*code),
        }
    }

    /// Get error message
    pub fn message(&self) -> String {
        match self {
            ZaiError::HttpError { message, .. } => message.clone(),
            ZaiError::WebSocketError { message, .. } => message.clone(),
            ZaiError::AuthError { message, .. } => message.clone(),
            ZaiError::AccountError { message, .. } => message.clone(),
            ZaiError::ApiError { message, .. } => message.clone(),
            ZaiError::RateLimitError { message, .. } => message.clone(),
            ZaiError::ContentPolicyError { message, .. } => message.clone(),
            ZaiError::FileError { message, .. } => message.clone(),
            ZaiError::NetworkError(err) => err.clone(),
            ZaiError::ConnectionError(err) => err.clone(),
            ZaiError::ProtocolError(err) => err.clone(),
            ZaiError::JsonError(err) => err.to_string(),
            ZaiError::TlsError(err) => err.clone(),
            ZaiError::TimeoutError(err) => err.clone(),
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
        } else if err.is_timeout() {
            ZaiError::TimeoutError(err.to_string())
        } else if err.is_connect() {
            ZaiError::ConnectionError(err.to_string())
        } else {
            ZaiError::NetworkError(err.to_string())
        }
    }
}

/// Convert from tokio_tungstenite::tungstenite::Error to ZaiError
impl From<tokio_tungstenite::tungstenite::Error> for ZaiError {
    fn from(err: tokio_tungstenite::tungstenite::Error) -> Self {
        use tokio_tungstenite::tungstenite::Error;

        match err {
            Error::ConnectionClosed => ZaiError::websocket_connection_error("Connection closed"),
            Error::AlreadyClosed => {
                ZaiError::websocket_connection_error("Connection already closed")
            }
            Error::Io(io_err) => ZaiError::NetworkError(io_err.to_string()),
            Error::Tls(tls_err) => ZaiError::tls(format!("TLS error: {}", tls_err)),
            Error::Capacity(msg) => ZaiError::ProtocolError(format!("Capacity error: {}", msg)),
            Error::Protocol(msg) => ZaiError::ProtocolError(format!("Protocol error: {}", msg)),
            Error::Utf8 => ZaiError::ProtocolError("UTF-8 encoding error".to_string()),
            Error::Url(url_err) => ZaiError::ConnectionError(format!("Invalid URL: {}", url_err)),
            // Note: Some variants may not exist in all versions of tokio-tungstenite
            _ => ZaiError::NetworkError(format!("WebSocket error: {}", err)),
        }
    }
}

/// Convert from tokio_tungstenite::tungstenite::http::Error to ZaiError
impl From<tokio_tungstenite::tungstenite::http::Error> for ZaiError {
    fn from(err: tokio_tungstenite::tungstenite::http::Error) -> Self {
        ZaiError::ConnectionError(format!("HTTP request error: {}", err))
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
        match err.kind() {
            std::io::ErrorKind::TimedOut => ZaiError::TimeoutError(err.to_string()),
            std::io::ErrorKind::ConnectionRefused => ZaiError::ConnectionError(err.to_string()),
            std::io::ErrorKind::ConnectionReset => ZaiError::ConnectionError(err.to_string()),
            std::io::ErrorKind::ConnectionAborted => ZaiError::ConnectionError(err.to_string()),
            std::io::ErrorKind::NotConnected => ZaiError::ConnectionError(err.to_string()),
            std::io::ErrorKind::BrokenPipe => ZaiError::ConnectionError(err.to_string()),
            std::io::ErrorKind::WouldBlock => ZaiError::NetworkError(err.to_string()),
            _ => ZaiError::NetworkError(err.to_string()),
        }
    }
}

/// Convert from tokio::time::error::Elapsed to ZaiError
impl From<tokio::time::error::Elapsed> for ZaiError {
    fn from(err: tokio::time::error::Elapsed) -> Self {
        ZaiError::TimeoutError(format!("Operation timed out: {:?}", err))
    }
}

/// Convert from Box<dyn std::error::Error + Send + Sync> to ZaiError
impl From<Box<dyn std::error::Error + Send + Sync>> for ZaiError {
    fn from(err: Box<dyn std::error::Error + Send + Sync>) -> Self {
        ZaiError::Unknown {
            code: 0,
            message: err.to_string(),
        }
    }
}
