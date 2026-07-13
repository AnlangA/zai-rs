//! # Error Types
//!
//! Defines the unified error type for the ZAI-RS SDK, mapping Zhipu AI API
//! error codes. See <https://docs.bigmodel.cn/cn/api/api-code> for the full
//! reference.
//!
//! # Error Categories
//!
//! | Variant | Code range | Description |
//! |---------|------------|-------------|
//! | [`ZaiError::AuthError`] | 1000, 1001, 1003, 1005, 1220 | Authentication / authorization (invalid API key, etc.) |
//! | [`ZaiError::AccountError`] | 1110–1121 except 1113 | Account/package-related errors |
//! | [`ZaiError::ApiError`] | 1200–1234, 1261 | Request validation or upstream execution errors |
//! | [`ZaiError::ContentPolicyError`] | 1301 | API policy / unsafe-content blocks |
//! | [`ZaiError::RateLimitError`] | 1113, 1302, 1305, 1308–1311, 1313–1321 | Rate-limit, quota, package pressure or fair-use errors |
//! | [`ZaiError::FileError`] | 1400–1499 | File-processing errors |
//! | [`ZaiError::Unknown`] | other | Unrecognized business or HTTP errors |
//! | [`ZaiError::NetworkError`] | — | Network / timeout errors |
//! | [`ZaiError::JsonError`] | — | JSON serialization / deserialization errors |
//!
//! # Sensitive-Data Masking
//!
//! [`mask_sensitive_info`] can sanitize text before a caller writes it to a log.
//! It is an explicit helper, not a logging hook: arbitrary log messages are not
//! automatically filtered by this module.
//!
//! # Example
//!
//! ```rust,no_run
//! use zai_rs::client::error::{ZaiError, ZaiResult};
//!
//! async fn call_api() -> ZaiResult<String> {
//!     Ok("result".to_string())
//! }
//!
//! # async fn example() {
//! match call_api().await {
//!     Ok(data) => println!("Success: {}", data),
//!     Err(ZaiError::AuthError { code, message }) => {
//!         tracing::error!("Auth failed ({}): {}", code, message);
//!     },
//!     Err(ZaiError::RateLimitError { code, message }) => {
//!         tracing::error!("Rate limited ({}): {}", code, message);
//!     },
//!     Err(e) => tracing::error!("Error: {}", e),
//! }
//! # }
//! ```

use std::sync::Arc;

use thiserror::Error;

/// Reserved error-code constants for failures originating inside the SDK.
pub mod codes;
mod redaction;

pub use redaction::{contains_sensitive_info, mask_api_key, mask_sensitive_info, validate_api_key};

/// Main error type for the ZAI-RS SDK
#[derive(Error, Debug, Clone)]
#[non_exhaustive]
pub enum ZaiError {
    /// HTTP status errors
    #[error("HTTP error [{status}]: {message}")]
    HttpError {
        /// HTTP status code (e.g. `400`, `404`, `500`).
        status: u16,
        /// Human-readable error message returned with the response.
        message: String,
    },

    /// Authentication and authorization errors
    #[error("Authentication error [{code}]: {message}")]
    AuthError {
        /// Zhipu AI authentication/authorization business error code.
        code: u16,
        /// Human-readable error message.
        message: String,
    },

    /// Account-related errors
    #[error("Account error [{code}]: {message}")]
    AccountError {
        /// Zhipu AI account business error code (`1110`–`1121`, excluding
        /// quota/billing code `1113`).
        code: u16,
        /// Human-readable error message.
        message: String,
    },

    /// API call errors
    #[error("API error [{code}]: {message}")]
    ApiError {
        /// Zhipu AI business error code (`1200`–`1234` or `1261`) or a
        /// reserved SDK code from [`codes`] (`9000`–`9999`). Business codes
        /// `1200`, `1230`, and `1234` are categorized as server failures.
        code: u16,
        /// Human-readable error message.
        message: String,
    },

    /// Rate limiting and quota errors
    #[error("Rate limit error [{code}]: {message}")]
    RateLimitError {
        /// Zhipu AI rate-limit, quota, or billing business error code.
        code: u16,
        /// Human-readable error message.
        message: String,
    },

    /// Content policy errors
    #[error("Content policy error [{code}]: {message}")]
    ContentPolicyError {
        /// Zhipu AI business error code `1301` for policy blocks or
        /// unsafe-content violations.
        code: u16,
        /// Human-readable error message.
        message: String,
    },

    /// File processing errors
    #[error("File error [{code}]: {message}")]
    FileError {
        /// Zhipu AI business error code (`1400`–`1499`) or a reserved SDK
        /// file code from [`codes`].
        code: u16,
        /// Human-readable error message.
        message: String,
    },

    /// Network/IO errors (wrapped in Arc for Clone support). The underlying
    /// `reqwest::Error` is exposed as the [`Error::source`](std::error::Error::source).
    #[error("Network error: {0}")]
    NetworkError(#[source] Arc<reqwest::Error>),

    /// JSON parsing errors (wrapped in Arc for Clone support). The underlying
    /// `serde_json::Error` is exposed as the [`Error::source`](std::error::Error::source).
    #[error("JSON error: {0}")]
    JsonError(#[source] Arc<serde_json::Error>),

    /// Realtime (WebSocket) transport errors — wrapped in `Arc` so the variant
    /// stays `Clone`-able. See [`RealtimeErrorKind`] for the breakdown. The
    /// kind is exposed as the [`Error::source`](std::error::Error::source).
    #[error("Realtime error: {0}")]
    RealtimeError(#[source] Arc<RealtimeErrorKind>),

    /// Realtime authentication / JWT errors (bad API-key shape, signing
    /// failure, token rejected during the WebSocket handshake).
    #[error("Realtime auth error: {0}")]
    RealtimeAuthError(String),

    /// Other errors
    #[error("Unknown error [{code}]: {message}")]
    Unknown {
        /// Numeric code — either an unmapped business code, an HTTP status,
        /// or a reserved SDK code from [`codes`].
        code: u16,
        /// Human-readable error message.
        message: String,
    },
}

/// Coarse classification of a [`ZaiError`] for retry/recovery decisions.
///
/// The single source of truth consulted by [`ZaiError::category`]; the
/// [`is_client_error`](ZaiError::is_client_error) /
/// [`is_server_error`](ZaiError::is_server_error) predicates derive from it so
/// they can never drift apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ErrorCategory {
    /// Caller-side (4xx): bad request, bad params, business-rule violation.
    Client,
    /// Server-side (5xx): transient backend failure.
    Server,
    /// Rate limiting / quota (HTTP 429 and the documented quota business
    /// codes).
    RateLimit,
    /// Network / transport failure (connection, timeout, WebSocket).
    Network,
    /// Authentication / authorization — re-auth, do not retry.
    Auth,
    /// (De)serialization of a payload — programmer error.
    Serialization,
    /// Anything not covered above.
    Other,
}

/// Map a raw HTTP/business status code to an [`ErrorCategory`].
fn classify_status(status: u16) -> ErrorCategory {
    match status {
        429 => ErrorCategory::RateLimit,
        s if (400..500).contains(&s) => ErrorCategory::Client,
        s if (500..600).contains(&s) && !matches!(s, 501 | 505) => ErrorCategory::Server,
        _ => ErrorCategory::Other,
    }
}

/// Concrete error categories for the realtime (WebSocket) transport.
///
/// Kept separate from [`ZaiError`] so callers can introspect the failure mode
/// without matching on the full enum, and so the realtime module can construct
/// rich errors without touching HTTP-specific machinery.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum RealtimeErrorKind {
    /// Low-level WebSocket error (connect/handshake/read/write). The original
    /// `tungstenite` error is kept as the `#[source]` so the full chain
    /// survives propagation. Only available with the `realtime` feature.
    #[cfg(feature = "realtime")]
    #[error("websocket: {source}")]
    WebSocket {
        /// The underlying tungstenite error.
        #[source]
        source: tokio_tungstenite::tungstenite::Error,
    },

    /// Protocol violation — unexpected or malformed server event.
    #[error("protocol: {0}")]
    Protocol(String),

    /// A realtime connect, write, or close operation exceeded its deadline.
    #[error("{operation} timed out")]
    Timeout {
        /// Operation whose deadline elapsed.
        operation: &'static str,
    },

    /// The WebSocket session has been closed.
    #[error("session closed")]
    Closed,
}

impl ZaiError {
    /// Convert an HTTP status code and API error response to a [`ZaiError`].
    pub fn from_api_response(status: u16, api_code: u16, api_message: String) -> Self {
        // Provider and proxy error bodies are untrusted and occasionally echo
        // request metadata. Remove recognizable credentials before the text
        // enters a public, loggable error value.
        let api_message = mask_sensitive_info(&api_message);
        if api_code != 0 {
            return match api_code {
                // Authentication errors
                1000 | 1001 | 1003 | 1005 | 1220 => ZaiError::AuthError {
                    code: api_code,
                    message: api_message,
                },
                // Billing exhaustion is surfaced with the other quota errors,
                // not as a generic account-state failure.
                1113 => ZaiError::RateLimitError {
                    code: api_code,
                    message: api_message,
                },
                // Account/package/balance errors
                1110..=1121 => ZaiError::AccountError {
                    code: api_code,
                    message: api_message,
                },
                // API call and validation errors. Code 1261 is the documented
                // context-window validation failure outside the main 12xx
                // range used by the other request errors.
                1200..=1234 | 1261 => ZaiError::ApiError {
                    code: api_code,
                    message: api_message,
                },
                // API policy and unsafe-content blocks are not transient.
                1301 => ZaiError::ContentPolicyError {
                    code: api_code,
                    message: api_message,
                },
                // Rate limiting, quota, package access pressure/fair-use errors.
                1302 | 1305 | 1308..=1311 | 1313..=1321 => ZaiError::RateLimitError {
                    code: api_code,
                    message: api_message,
                },
                // File processing errors
                1400..=1499 => ZaiError::FileError {
                    code: api_code,
                    message: api_message,
                },
                _ => ZaiError::Unknown {
                    code: api_code,
                    message: message_or(api_message, "Unknown error"),
                },
            };
        }

        // Fall back to HTTP status when no business code is present. Every 5xx
        // remains an HttpError carrying the real status; 401/403 are classified
        // as authentication errors and 429 as rate limiting.
        match status {
            400 => ZaiError::HttpError {
                status,
                message: message_or(api_message, "Bad request - check your parameters"),
            },
            401 | 403 => ZaiError::AuthError {
                code: status,
                message: message_or(api_message, "Unauthorized - check your API key"),
            },
            404 => ZaiError::HttpError {
                status,
                message: message_or(api_message, "Not found - requested resource doesn't exist"),
            },
            429 => ZaiError::RateLimitError {
                code: status,
                message: message_or(api_message, "Too many requests - rate limit exceeded"),
            },
            434 => ZaiError::HttpError {
                status,
                message: message_or(api_message, "No API permission - feature not available"),
            },
            435 => ZaiError::HttpError {
                status,
                message: message_or(api_message, "File size exceeds 100MB limit"),
            },
            // All 5xx keep the status so classification can use it directly.
            s if (500..600).contains(&s) => ZaiError::HttpError {
                status,
                message: if api_message.is_empty() {
                    format!("Server error (HTTP {status}) - try again later")
                } else {
                    api_message
                },
            },
            s if (400..500).contains(&s) => ZaiError::HttpError {
                status,
                message: message_or(api_message, "HTTP client error"),
            },
            _ => ZaiError::Unknown {
                code: status,
                message: message_or(api_message, "Unknown error"),
            },
        }
    }

    /// Check if the error is a rate limit error
    pub fn is_rate_limit(&self) -> bool {
        self.category() == ErrorCategory::RateLimit
    }

    /// Check if the error is an authentication error
    pub fn is_auth_error(&self) -> bool {
        self.category() == ErrorCategory::Auth
    }

    /// Classify this error into a single canonical [`ErrorCategory`].
    ///
    /// This is the one place the SDK decides whether an error is client-side,
    /// server-side, a rate limit, a network blip, etc. The convenience
    /// predicates ([`is_client_error`](Self::is_client_error),
    /// [`is_server_error`](Self::is_server_error)) derive from it, so they can
    /// never disagree.
    pub fn category(&self) -> ErrorCategory {
        match self {
            ZaiError::RateLimitError { .. } => ErrorCategory::RateLimit,
            ZaiError::NetworkError(_) => ErrorCategory::Network,
            ZaiError::ApiError { code, .. } if *code == codes::SDK_TIMEOUT => {
                ErrorCategory::Network
            },
            ZaiError::AuthError { .. } | ZaiError::RealtimeAuthError(_) => ErrorCategory::Auth,
            ZaiError::ApiError { code, .. } if is_server_business_code(*code) => {
                ErrorCategory::Server
            },
            ZaiError::AccountError { .. }
            | ZaiError::ApiError { .. }
            | ZaiError::ContentPolicyError { .. }
            | ZaiError::FileError { .. } => ErrorCategory::Client,
            ZaiError::JsonError(_) => ErrorCategory::Serialization,
            ZaiError::RealtimeError(kind) => match kind.as_ref() {
                // Protocol failures are client-caused; transport timeouts and
                // WebSocket failures are network-level; closure is neither.
                RealtimeErrorKind::Protocol(_) => ErrorCategory::Client,
                #[cfg(feature = "realtime")]
                RealtimeErrorKind::WebSocket { .. } => ErrorCategory::Network,
                RealtimeErrorKind::Timeout { .. } => ErrorCategory::Network,
                RealtimeErrorKind::Closed => ErrorCategory::Other,
            },
            ZaiError::HttpError { status, .. } => classify_status(*status),
            // `Unknown` mirrors an unmapped HTTP/business code: 5xx is a server
            // error, everything else is uncategorized. It is intentionally *not*
            // classified as client-side or rate-limited even at 4xx/429, matching
            // the legacy predicate behavior (and it is not retried — its
            // transience is uncertain).
            ZaiError::Unknown { code, .. } => {
                if (500..600).contains(code) {
                    ErrorCategory::Server
                } else {
                    ErrorCategory::Other
                }
            },
        }
    }

    /// Check if the error is a client error (4xx), including auth and rate
    /// limiting (which arrive as 4xx responses).
    pub fn is_client_error(&self) -> bool {
        matches!(
            self.category(),
            ErrorCategory::Client | ErrorCategory::Auth | ErrorCategory::RateLimit
        )
    }

    /// Check if the error is a server error (5xx).
    pub fn is_server_error(&self) -> bool {
        matches!(self.category(), ErrorCategory::Server)
    }

    /// Whether retrying the request that produced this error could succeed.
    ///
    /// This caller-facing helper marks transient rate limits (`429`, `1302`,
    /// `1305`), documented upstream execution failures, network failures, and
    /// HTTP 5xx errors as potentially retryable. Quota/billing exhaustion is
    /// deliberately not retryable. This method does not account for request
    /// idempotency or attempt limits; the internal HTTP transport applies those
    /// additional constraints.
    pub fn is_retryable(&self) -> bool {
        match self {
            ZaiError::HttpError { status, .. } => {
                crate::client::transport::retry::RETRYABLE_STATUSES.contains(status)
            },
            ZaiError::RateLimitError { code, .. } => matches!(*code, 429 | 1302 | 1305),
            ZaiError::NetworkError(_) => true,
            ZaiError::ApiError { code, .. } if *code == codes::SDK_TIMEOUT => true,
            ZaiError::ApiError { code, .. } if is_server_business_code(*code) => true,
            ZaiError::RealtimeError(kind)
                if matches!(kind.as_ref(), RealtimeErrorKind::Timeout { .. }) =>
            {
                true
            },
            #[cfg(feature = "realtime")]
            ZaiError::RealtimeError(kind)
                if matches!(kind.as_ref(), RealtimeErrorKind::WebSocket { .. }) =>
            {
                true
            },
            _ => false,
        }
    }

    /// Whether this error originates from the SDK itself rather than the API.
    ///
    /// True iff [`code`](Self::code) is in the reserved `9000`–`9999` band
    /// (see [`codes`]). Variants without a numeric code
    /// ([`NetworkError`](Self::NetworkError), [`JsonError`](Self::JsonError),
    /// [`RealtimeError`](Self::RealtimeError)) return `false`.
    pub fn is_sdk_error(&self) -> bool {
        self.code().is_some_and(|c| (9000..=9999).contains(&c))
    }

    /// Get a compact representation of error suitable for logging
    pub fn compact(&self) -> String {
        match self {
            ZaiError::HttpError { status, message } => {
                format!("HTTP[{status}]: {message}")
            },
            ZaiError::AuthError { code, message } => {
                format!("AUTH[{code}]: {message}")
            },
            ZaiError::AccountError { code, message } => {
                format!("ACCOUNT[{code}]: {message}")
            },
            ZaiError::ApiError { code, message } => {
                format!("API[{code}]: {message}")
            },
            ZaiError::RateLimitError { code, message } => {
                format!("RATE_LIMIT[{code}]: {message}")
            },
            ZaiError::ContentPolicyError { code, message } => {
                format!("POLICY[{code}]: {message}")
            },
            ZaiError::FileError { code, message } => {
                format!("FILE[{code}]: {message}")
            },
            ZaiError::NetworkError(err) => {
                format!("NETWORK: {err}")
            },
            ZaiError::JsonError(err) => {
                format!("JSON: {err}")
            },
            ZaiError::RealtimeError(kind) => {
                format!("REALTIME: {kind}")
            },
            ZaiError::RealtimeAuthError(msg) => {
                format!("REALTIME_AUTH: {msg}")
            },
            ZaiError::Unknown { code, message } => {
                format!("UNKNOWN[{code}]: {message}")
            },
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
            ZaiError::RealtimeError(_) | ZaiError::RealtimeAuthError(_) => None,
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
            ZaiError::RealtimeError(kind) => kind.to_string(),
            ZaiError::RealtimeAuthError(msg) => msg.clone(),
            ZaiError::Unknown { message, .. } => message.clone(),
        }
    }

    /// Attach an operational context to this error without losing its code or
    /// category.
    ///
    /// Prepends `"{context}: "` to the human-readable message of every variant
    /// that carries one. Variants whose payload is a wrapped source error with
    /// no message slot ([`NetworkError`](Self::NetworkError),
    /// [`JsonError`](Self::JsonError), [`RealtimeError`](Self::RealtimeError))
    /// are returned unchanged — record their context in a `tracing` span
    /// instead.
    ///
    /// # Example
    ///
    /// ```
    /// use zai_rs::client::error::ZaiError;
    ///
    /// let err = ZaiError::ApiError {
    ///     code: 1200,
    ///     message: "bad model".to_string(),
    /// };
    /// let ctx = err.context("file parser create");
    /// assert_eq!(ctx.code(), Some(1200));
    /// assert_eq!(ctx.message(), "file parser create: bad model");
    /// ```
    pub fn context(self, context: &str) -> Self {
        let with_context = |message: String| format!("{context}: {message}");
        match self {
            Self::HttpError { status, message } => Self::HttpError {
                status,
                message: with_context(message),
            },
            Self::AuthError { code, message } => Self::AuthError {
                code,
                message: with_context(message),
            },
            Self::AccountError { code, message } => Self::AccountError {
                code,
                message: with_context(message),
            },
            Self::ApiError { code, message } => Self::ApiError {
                code,
                message: with_context(message),
            },
            Self::RateLimitError { code, message } => Self::RateLimitError {
                code,
                message: with_context(message),
            },
            Self::ContentPolicyError { code, message } => Self::ContentPolicyError {
                code,
                message: with_context(message),
            },
            Self::FileError { code, message } => Self::FileError {
                code,
                message: with_context(message),
            },
            // No message slot: keep the wrapped source as-is (context belongs
            // in a tracing span, not by flattening the source to a string).
            Self::NetworkError(err) => Self::NetworkError(err),
            Self::JsonError(err) => Self::JsonError(err),
            Self::RealtimeError(kind) => Self::RealtimeError(kind),
            Self::RealtimeAuthError(message) => Self::RealtimeAuthError(with_context(message)),
            Self::Unknown { code, message } => Self::Unknown {
                code,
                message: with_context(message),
            },
        }
    }
}

/// Business errors documented by the upstream API as server-side execution
/// failures even though they share the broad `12xx` API-error namespace.
const fn is_server_business_code(code: u16) -> bool {
    matches!(code, 1200 | 1230 | 1234)
}

fn message_or(message: String, fallback: &str) -> String {
    if message.is_empty() {
        fallback.to_string()
    } else {
        message
    }
}

/// Type alias for Result with ZaiError
pub type ZaiResult<T> = Result<T, ZaiError>;

/// Convert from reqwest::Error to ZaiError
impl From<reqwest::Error> for ZaiError {
    fn from(err: reqwest::Error) -> Self {
        // Reqwest attaches the request URL to many errors. Strip it before the
        // error enters the public type so paths and query values cannot leak
        // through Display, Debug, `compact()`, or tracing.
        let err = err.without_url();
        if let Some(status) = err.status() {
            ZaiError::from_api_response(status.as_u16(), 0, err.to_string())
        } else {
            ZaiError::NetworkError(Arc::new(err))
        }
    }
}

/// Convert from serde_json::Error to ZaiError
impl From<serde_json::Error> for ZaiError {
    fn from(err: serde_json::Error) -> Self {
        ZaiError::JsonError(Arc::new(err))
    }
}

/// Convert from validator::ValidationErrors to ZaiError
impl From<validator::ValidationErrors> for ZaiError {
    fn from(err: validator::ValidationErrors) -> Self {
        ZaiError::ApiError {
            code: codes::SDK_VALIDATION,
            message: sanitized_validation_message(&err),
        }
    }
}

fn sanitized_validation_message(errors: &validator::ValidationErrors) -> String {
    fn collect(errors: &validator::ValidationErrors, prefix: &str, output: &mut Vec<String>) {
        use validator::ValidationErrorsKind;

        for (field, kind) in errors.errors() {
            let path = if prefix.is_empty() {
                field.to_string()
            } else {
                format!("{prefix}.{field}")
            };
            match kind {
                ValidationErrorsKind::Field(field_errors) => {
                    for error in field_errors {
                        let mut constraints = error
                            .params
                            .keys()
                            .filter(|name| name.as_ref() != "value")
                            .map(ToString::to_string)
                            .collect::<Vec<_>>();
                        constraints.sort_unstable();
                        if constraints.is_empty() {
                            output.push(format!("{path}:{}", error.code));
                        } else {
                            output.push(format!(
                                "{path}:{} ({})",
                                error.code,
                                constraints.join(",")
                            ));
                        }
                    }
                },
                ValidationErrorsKind::Struct(nested) => collect(nested, &path, output),
                ValidationErrorsKind::List(items) => {
                    for (index, nested) in items {
                        collect(nested, &format!("{path}[{index}]"), output);
                    }
                },
            }
        }
    }

    let mut issues = Vec::new();
    collect(errors, "", &mut issues);
    issues.sort_unstable();
    if issues.is_empty() {
        "validation failed".to_owned()
    } else {
        format!("validation failed: {}", issues.join("; "))
    }
}

/// Convert from std::io::Error to ZaiError.
///
/// Maps by [`std::io::ErrorKind`] so the category (file vs. timeout vs.
/// generic I/O) survives propagation instead of collapsing to a single
/// opaque `Unknown{0}`. A `NetworkError` cannot be built from an
/// `io::Error` (it wraps `reqwest::Error`), so `TimedOut` is reported as an
/// [`ApiError`](Self::ApiError) carrying [`codes::SDK_TIMEOUT`].
impl From<std::io::Error> for ZaiError {
    fn from(err: std::io::Error) -> Self {
        use std::io::ErrorKind;
        match err.kind() {
            ErrorKind::NotFound => ZaiError::FileError {
                code: codes::SDK_FILE_NOT_FOUND,
                message: err.to_string(),
            },
            ErrorKind::PermissionDenied => ZaiError::FileError {
                code: codes::SDK_IO,
                message: err.to_string(),
            },
            ErrorKind::TimedOut => ZaiError::ApiError {
                code: codes::SDK_TIMEOUT,
                message: err.to_string(),
            },
            _ => ZaiError::Unknown {
                code: codes::SDK_IO,
                message: err.to_string(),
            },
        }
    }
}

/// Convert from a realtime transport error kind into a [`ZaiError`].
impl From<RealtimeErrorKind> for ZaiError {
    fn from(kind: RealtimeErrorKind) -> Self {
        ZaiError::RealtimeError(Arc::new(kind))
    }
}

/// Convert from a low-level WebSocket (`tungstenite`) error into a
/// [`ZaiError`]. The original error is preserved as the `#[source]` of
/// [`RealtimeErrorKind::WebSocket`]. Only available with the `realtime`
/// feature.
#[cfg(feature = "realtime")]
impl From<tokio_tungstenite::tungstenite::Error> for ZaiError {
    fn from(err: tokio_tungstenite::tungstenite::Error) -> Self {
        ZaiError::RealtimeError(Arc::new(RealtimeErrorKind::WebSocket { source: err }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // `super::*` would otherwise pull in the `thiserror::Error` derive macro as
    // `Error`; bring `std::io`'s `Error`/`ErrorKind` into scope for the io-Error
    // conversion tests below.
    use std::io::{Error, ErrorKind};
    use validator::Validate;

    #[derive(Validate)]
    struct SensitiveValidationInput {
        #[validate(length(min = 64))]
        prompt: String,
    }

    #[test]
    fn validation_conversion_never_includes_the_rejected_value() {
        let sensitive = "private customer prompt";
        let errors = SensitiveValidationInput {
            prompt: sensitive.to_owned(),
        }
        .validate()
        .expect_err("the short prompt must fail the test validator");
        let error = ZaiError::from(errors);
        let rendered = error.to_string();

        assert!(rendered.contains("prompt:length"));
        assert!(rendered.contains("min"));
        assert!(!rendered.contains(sensitive));
    }

    #[test]
    fn test_from_api_response_bad_request() {
        let err = ZaiError::from_api_response(400, 0, "Invalid input".to_string());
        assert!(err.is_client_error());
        assert!(!err.is_server_error());
        assert_eq!(err.code(), Some(400));
    }

    #[test]
    fn api_response_messages_are_credential_redacted() {
        let error = ZaiError::from_api_response(
            400,
            1210,
            "authorization: Bearer abc123.abcdefghijklmnopqrstuvwxyz".to_owned(),
        );
        let rendered = error.to_string();
        assert!(!rendered.contains("abcdefghijklmnopqrstuvwxyz"));
        assert!(rendered.contains("[AUTH_REDACTED]"));
    }

    #[test]
    fn test_from_api_response_unauthorized() {
        let err = ZaiError::from_api_response(401, 0, "".to_string());
        assert!(err.is_client_error());
        assert_eq!(err.message(), "Unauthorized - check your API key");
    }

    #[test]
    fn test_from_api_response_rate_limit() {
        // Business code takes precedence over HTTP status.
        let err = ZaiError::from_api_response(429, 1302, "Too many requests".to_string());
        assert!(err.is_client_error());
        assert!(err.is_rate_limit());
        assert_eq!(err.code(), Some(1302));

        // API code 1302 returns RateLimitError even with a non-error HTTP
        // status.
        let err = ZaiError::from_api_response(200, 1302, "Too many requests".to_string());
        assert!(err.is_client_error());
        assert!(err.is_rate_limit());
        assert_eq!(err.code(), Some(1302));
    }

    #[test]
    fn test_from_api_response_package_limit_codes() {
        for code in [
            1113, 1302, 1305, 1308, 1309, 1310, 1311, 1313, 1314, 1315, 1316, 1317, 1318, 1319,
            1320, 1321,
        ] {
            let err = ZaiError::from_api_response(429, code, "Limited".to_string());
            assert!(err.is_rate_limit());
            assert_eq!(err.code(), Some(code));
        }

        for code in [1113, 1308, 1314, 1321] {
            let err = ZaiError::from_api_response(429, code, "Limited".to_string());
            assert!(!err.is_retryable(), "quota code {code} must not retry");
        }
    }

    #[test]
    fn test_from_api_response_content_policy_codes() {
        let err = ZaiError::from_api_response(400, 1301, "Blocked".to_string());
        assert!(matches!(err, ZaiError::ContentPolicyError { .. }));
        assert!(err.is_client_error());
        assert!(!err.is_rate_limit());
        assert_eq!(err.code(), Some(1301));
    }

    #[test]
    fn context_window_code_is_a_non_retryable_client_validation_error() {
        let error =
            ZaiError::from_api_response(400, 1261, "model context window exceeded".to_owned());
        assert!(matches!(error, ZaiError::ApiError { code: 1261, .. }));
        assert_eq!(error.category(), ErrorCategory::Client);
        assert!(!error.is_retryable());
    }

    #[test]
    fn test_from_api_response_server_error() {
        let err = ZaiError::from_api_response(500, 0, "".to_string());
        assert!(!err.is_client_error());
        assert!(err.is_server_error());
    }

    #[test]
    fn test_from_api_response_auth_error_code() {
        for code in [1001, 1005, 1220] {
            let err = ZaiError::from_api_response(200, code, "Invalid API key".to_string());
            assert!(err.is_auth_error());
            assert_eq!(err.code(), Some(code));
            assert_eq!(err.message(), "Invalid API key");
        }
    }

    #[test]
    fn test_from_api_response_account_error() {
        let err = ZaiError::from_api_response(200, 1110, "Account expired".to_string());
        assert!(err.is_client_error());
        assert_eq!(err.code(), Some(1110));
    }

    #[test]
    fn test_from_api_response_api_error() {
        let err = ZaiError::from_api_response(200, 1210, "Invalid parameters".to_string());
        assert!(err.is_client_error());
        assert_eq!(err.code(), Some(1210));
    }

    #[test]
    fn server_business_codes_preserve_code_and_retryability() {
        for code in [1200, 1230, 1234] {
            let err = ZaiError::from_api_response(200, code, "upstream failed".to_string());
            assert!(matches!(err, ZaiError::ApiError { .. }));
            assert_eq!(err.code(), Some(code));
            assert_eq!(err.category(), ErrorCategory::Server);
            assert!(err.is_server_error());
            assert!(err.is_retryable());
            assert!(!err.is_client_error());
        }
    }

    #[test]
    fn test_from_api_response_unknown_code() {
        let err = ZaiError::from_api_response(200, 9999, "Unknown error".to_string());
        assert!(!err.is_client_error()); // Unknown code doesn't mean client error
        assert_eq!(err.code(), Some(9999));
    }

    #[test]
    fn test_compact() {
        let err = ZaiError::HttpError {
            status: 404,
            message: "Not found".to_string(),
        };
        assert_eq!(err.compact(), "HTTP[404]: Not found");

        let err = ZaiError::AuthError {
            code: 1001,
            message: "Invalid key".to_string(),
        };
        assert_eq!(err.compact(), "AUTH[1001]: Invalid key");
    }

    #[test]
    fn test_code() {
        // Using From trait implementation for io::Error: ErrorKind::ConnectionRefused
        // is not NotFound/PermissionDenied/TimedOut, so it falls through to
        // Unknown carrying the SDK I/O code.
        let io_err = Error::new(ErrorKind::ConnectionRefused, "connection refused");
        let err = ZaiError::from(io_err);
        assert_eq!(err.code(), Some(codes::SDK_IO));

        // JsonError has no code
        let err = ZaiError::JsonError(Arc::new(serde_json::Error::io(Error::new(
            ErrorKind::InvalidData,
            "invalid JSON",
        ))));
        assert!(err.code().is_none());

        // HttpError has status as code
        let err = ZaiError::HttpError {
            status: 500,
            message: "Server error".to_string(),
        };
        assert_eq!(err.code(), Some(500));
    }

    #[test]
    fn test_message() {
        let err = ZaiError::RateLimitError {
            code: 1302,
            message: "Too many requests".to_string(),
        };
        assert_eq!(err.message(), "Too many requests");
    }

    #[test]
    fn test_from_reqwest_error_with_status() {
        let io_err = Error::other("test error");
        let zai_err = ZaiError::from(io_err);
        match zai_err {
            ZaiError::Unknown { .. } => {},
            _ => panic!("Expected Unknown error for io::Error"),
        }
    }

    #[test]
    fn test_sdk_code_constants_in_reserved_range() {
        for code in [
            codes::SDK_VALIDATION,
            codes::SDK_CONFIG,
            codes::SDK_FILE_NOT_FOUND,
            codes::SDK_FILE_TOO_LARGE,
            codes::SDK_FILE_TYPE_UNSUPPORTED,
            codes::SDK_IO,
            codes::SDK_TIMEOUT,
            codes::SDK_EXTERNAL_TOOL,
        ] {
            assert!(
                (9000..=9999).contains(&code),
                "code {code} outside 9000-9999"
            );
        }
    }

    #[test]
    fn test_is_sdk_error_classification() {
        // SDK codes → true.
        assert!(
            ZaiError::FileError {
                code: codes::SDK_FILE_NOT_FOUND,
                message: "x".into(),
            }
            .is_sdk_error()
        );
        assert!(
            ZaiError::ApiError {
                code: codes::SDK_TIMEOUT,
                message: "x".into(),
            }
            .is_sdk_error()
        );

        // API / HTTP codes → false.
        assert!(
            !ZaiError::AuthError {
                code: 1001,
                message: "x".into(),
            }
            .is_sdk_error()
        );
        assert!(
            !ZaiError::RateLimitError {
                code: 1302,
                message: "x".into(),
            }
            .is_sdk_error()
        );
        assert!(
            !ZaiError::HttpError {
                status: 500,
                message: "x".into(),
            }
            .is_sdk_error()
        );

        // Code-less variants → false.
        assert!(!ZaiError::RealtimeAuthError("x".into()).is_sdk_error());
    }

    #[test]
    fn test_from_io_maps_by_kind() {
        use std::io::{Error, ErrorKind};

        let err = ZaiError::from(Error::from(ErrorKind::NotFound));
        assert!(matches!(
            err,
            ZaiError::FileError { code, .. } if code == codes::SDK_FILE_NOT_FOUND
        ));

        let err = ZaiError::from(Error::from(ErrorKind::TimedOut));
        assert!(matches!(
            err,
            ZaiError::ApiError { code, .. } if code == codes::SDK_TIMEOUT
        ));

        let err = ZaiError::from(Error::from(ErrorKind::PermissionDenied));
        assert!(matches!(
            err,
            ZaiError::FileError { code, .. } if code == codes::SDK_IO
        ));

        // Unmapped kind → Unknown with SDK_IO code (no longer code 0).
        let err = ZaiError::from(Error::other("boom"));
        assert!(matches!(
            err,
            ZaiError::Unknown { code, .. } if code == codes::SDK_IO
        ));
    }

    #[test]
    fn test_context_preserves_code_and_variant() {
        let err = ZaiError::ApiError {
            code: 1200,
            message: "bad model".into(),
        }
        .context("file parser create");
        assert!(matches!(
            err,
            ZaiError::ApiError { code, .. } if code == 1200
        ));
        assert_eq!(err.message(), "file parser create: bad model");

        let err = ZaiError::Unknown {
            code: codes::SDK_IO,
            message: "boom".into(),
        }
        .context("read");
        assert_eq!(err.code(), Some(codes::SDK_IO));
        assert_eq!(err.message(), "read: boom");
    }

    #[test]
    fn test_sdk_timeout_is_not_rate_limit() {
        // Regression guard: a client-side polling timeout must NOT masquerade
        // as a rate-limit error (the previous implementation returned
        // RateLimitError{code:0}).
        let err = ZaiError::ApiError {
            code: codes::SDK_TIMEOUT,
            message: "Timeout waiting for parsing result".into(),
        };
        assert!(!err.is_rate_limit());
        assert!(err.is_sdk_error());
        assert_eq!(err.category(), ErrorCategory::Network);
        assert!(err.is_retryable());
        assert!(!err.is_client_error());
    }

    #[test]
    fn test_validate_api_key_valid() {
        assert!(validate_api_key("abc123.abcdefghijklmnopqrstuvwxyz").is_ok());
    }

    #[test]
    fn test_validate_api_key_empty() {
        let result = validate_api_key("");
        assert!(result.is_err());
        match result {
            Err(ZaiError::ApiError { code, .. }) => {
                assert_eq!(code, codes::SDK_VALIDATION);
            },
            _ => panic!("Expected ApiError"),
        }
    }

    #[test]
    fn test_validate_api_key_no_dot() {
        let result = validate_api_key("invalid");
        assert!(result.is_err());
        match result {
            Err(ZaiError::ApiError { code, message }) => {
                assert_eq!(code, codes::SDK_VALIDATION);
                assert!(message.contains("format"));
            },
            _ => panic!("Expected ApiError"),
        }
    }

    #[test]
    fn test_validate_api_key_multiple_dots() {
        let result = validate_api_key("id.secret.extra");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code(), Some(codes::SDK_VALIDATION));
    }

    #[test]
    fn test_validate_api_key_empty_id() {
        let result = validate_api_key(".secret123456789");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code(), Some(codes::SDK_VALIDATION));
    }

    #[test]
    fn test_validate_api_key_empty_secret() {
        let result = validate_api_key("id123.");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code(), Some(codes::SDK_VALIDATION));
    }

    #[test]
    fn test_validate_api_key_invalid_chars() {
        let result = validate_api_key("id$123.secret@456");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code(), Some(codes::SDK_VALIDATION));
    }

    #[test]
    fn test_validate_api_key_id_too_short() {
        let result = validate_api_key("ab.abcdefghijklmn");
        assert!(result.is_err());
        assert!(result.unwrap_err().message().contains("id is too short"));
    }

    #[test]
    fn test_validate_api_key_secret_too_short() {
        let result = validate_api_key("id123.short");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .message()
                .contains("secret is too short")
        );
    }

    #[test]
    fn test_mask_sensitive_info_api_key() {
        let text = "API key: abc123.abcdefghijklmnopqrstuvwxyz12345";
        let filtered = mask_sensitive_info(text);
        assert!(filtered.contains("[FILTERED]"));
        assert!(!filtered.contains("abc123"));
        assert!(!filtered.contains("abcdefghijklmnopqrstuvwxyz"));
    }

    #[test]
    fn test_mask_sensitive_info_password() {
        let text = "password: secret123, other text";
        let filtered = mask_sensitive_info(text);
        assert!(filtered.contains("[FILTERED]"));
        assert!(!filtered.contains("secret123"));
    }

    #[test]
    fn test_mask_sensitive_info_token() {
        let text = "token=abc123xyz, other content";
        let filtered = mask_sensitive_info(text);
        assert!(filtered.contains("[FILTERED]"));
        assert!(!filtered.contains("abc123xyz"));
    }

    #[test]
    fn test_mask_sensitive_info_bearer() {
        let text = "Authorization: Bearer abc123.abc1234567890";
        let filtered = mask_sensitive_info(text);
        // The whole Authorization header is replaced, including the header name
        // and Bearer scheme.
        assert!(filtered.contains("[AUTH_REDACTED]"));
        assert!(!filtered.contains("abc123"));
        assert!(!filtered.contains("Bearer"));
        assert!(!filtered.contains("Authorization"));
    }

    #[test]
    fn test_mask_standalone_bearer_jwt_and_opaque_token() {
        let jwt = "Bearer header.payload.signature";
        let opaque = "bearer opaque-token-value";
        assert_eq!(mask_sensitive_info(jwt), "Bearer [FILTERED]");
        assert_eq!(mask_sensitive_info(opaque), "bearer [FILTERED]");
        assert!(contains_sensitive_info(jwt));
        assert!(contains_sensitive_info(opaque));
    }

    #[test]
    fn test_mask_sensitive_info_multiple() {
        let text = "api_key=abc123.xyz456, password=secret123";
        let filtered = mask_sensitive_info(text);
        let filtered_count = filtered.matches("[FILTERED]").count();
        assert_eq!(filtered_count, 2);
    }

    #[test]
    fn test_mask_sensitive_info_no_sensitive() {
        let text = "Regular text without sensitive information";
        let filtered = mask_sensitive_info(text);
        assert_eq!(filtered, text);
    }

    #[test]
    fn test_mask_api_key() {
        let text = "API key: abc123.abcdefghijklmnopqrstuvwxyz12345";
        let filtered = mask_api_key(text);
        assert!(filtered.contains("[FILTERED]"));
        assert!(!filtered.contains("abc123"));
    }

    #[test]
    fn test_contains_sensitive_info_api_key() {
        assert!(contains_sensitive_info("api_key: abc123.abc1234567890"));
        assert!(contains_sensitive_info("-id.secret-value-"));
        assert!(!contains_sensitive_info("regular text"));
    }

    #[test]
    fn api_key_masking_handles_hyphen_boundaries_and_multiple_values() {
        let filtered = mask_sensitive_info("keys=-id.secret-value-,-next.another-secret-value-");
        assert_eq!(filtered.matches("[FILTERED]").count(), 2);
        assert!(!filtered.contains("secret-value"));
        assert!(!contains_sensitive_info(&filtered));
    }

    #[test]
    fn test_contains_sensitive_info_password() {
        assert!(contains_sensitive_info("password: secret"));
        assert!(contains_sensitive_info("password=123"));
        assert!(!contains_sensitive_info("password"));
        assert!(!contains_sensitive_info("word:password"));
    }

    #[test]
    fn test_contains_sensitive_info_token() {
        assert!(contains_sensitive_info("token=abc123"));
        assert!(contains_sensitive_info("token: xyz123"));
        assert!(!contains_sensitive_info("token"));
        assert!(!contains_sensitive_info("tokenize this"));
    }

    #[test]
    fn test_error_category_classification() {
        // Single source of truth: `category()` drives is_client_error /
        // is_server_error / is_retryable. Spot-check the classification table.

        // Rate-limit business error: client-side AND retryable.
        let rl = ZaiError::RateLimitError {
            code: 1302,
            message: "slow down".into(),
        };
        assert_eq!(rl.category(), ErrorCategory::RateLimit);
        assert!(rl.is_retryable());
        assert!(rl.is_client_error());
        assert!(!rl.is_server_error());

        // HTTP 429 -> rate limit, retryable, client-side.
        let h429 = ZaiError::HttpError {
            status: 429,
            message: "too many".into(),
        };
        assert_eq!(h429.category(), ErrorCategory::RateLimit);
        assert!(h429.is_retryable());
        assert!(h429.is_client_error());

        // HTTP 500 -> server, retryable, not client.
        let h500 = ZaiError::HttpError {
            status: 500,
            message: "boom".into(),
        };
        assert_eq!(h500.category(), ErrorCategory::Server);
        assert!(h500.is_retryable());
        assert!(h500.is_server_error());
        assert!(!h500.is_client_error());

        // HTTP 400 -> client, NOT retryable.
        let h400 = ZaiError::HttpError {
            status: 400,
            message: "bad".into(),
        };
        assert_eq!(h400.category(), ErrorCategory::Client);
        assert!(!h400.is_retryable());
        assert!(h400.is_client_error());

        // Auth -> client-side, not retryable.
        let auth = ZaiError::AuthError {
            code: 1001,
            message: "bad key".into(),
        };
        assert_eq!(auth.category(), ErrorCategory::Auth);
        assert!(!auth.is_retryable());
        assert!(auth.is_client_error());

        // Unknown 5xx is reported as a server error but, by design, is NOT
        // retried (its transience is uncertain) — this is the one intentional
        // divergence between `is_server_error` and `is_retryable`.
        let unk = ZaiError::Unknown {
            code: 503,
            message: "?".into(),
        };
        assert_eq!(unk.category(), ErrorCategory::Server);
        assert!(unk.is_server_error());
        assert!(!unk.is_retryable());
    }

    #[test]
    fn test_business_code_band_boundaries() {
        // Undocumented gaps remain Unknown instead of being absorbed by broad
        // numeric ranges.
        for code in [1002, 1004, 1100, 1300, 1303, 1304, 1306, 1307, 1312] {
            let e = ZaiError::from_api_response(400, code, "gap".to_string());
            assert!(
                matches!(e, ZaiError::Unknown { .. }),
                "code {code} -> Unknown"
            );
            assert!(!e.is_rate_limit());
        }
        // 1499 is the inclusive top of the FileError band (1400-1499).
        let e = ZaiError::from_api_response(400, 1499, "file".to_string());
        assert!(matches!(e, ZaiError::FileError { code, .. } if code == 1499));
        // 1400 is the bottom of the FileError band.
        let e = ZaiError::from_api_response(400, 1400, "file".to_string());
        assert!(matches!(e, ZaiError::FileError { code, .. } if code == 1400));
        // Documented rate-limit band edges and billing code are rate-limit errors.
        for code in [1113, 1302, 1305, 1308, 1321] {
            let e = ZaiError::from_api_response(429, code, "rl".to_string());
            assert!(e.is_rate_limit(), "code {code} -> RateLimitError");
        }
    }

    // ----- HTTP status classification for status-only responses -----

    #[test]
    fn status_502_503_504_classify_as_server_and_carry_status() {
        // Gateway/server failures retain their real status and classify as
        // server-side and retryable.
        for status in [502, 503, 504] {
            let e = ZaiError::from_api_response(status, 0, String::new());
            match &e {
                ZaiError::HttpError {
                    status: s,
                    message: _,
                } => {
                    assert_eq!(*s, status, "HTTP {status} lost its status code");
                    assert!(e.is_server_error(), "HTTP {status} not classified Server");
                    assert!(
                        e.is_retryable(),
                        "HTTP {status} should be retryable as a 5xx"
                    );
                },
                other => panic!("HTTP {status} classified as {other:?}, expected HttpError"),
            }
        }
        // 500 stays Server too.
        let e = ZaiError::from_api_response(500, 0, String::new());
        assert!(matches!(e, ZaiError::HttpError { status: 500, .. }));
        assert!(e.is_server_error());
    }

    #[test]
    fn retry_helper_uses_the_transport_status_matrix() {
        for status in [408, 425, 429, 500, 502, 503, 504] {
            let error = ZaiError::from_api_response(status, 0, String::new());
            assert!(error.is_retryable(), "HTTP {status} should be retryable");
        }
        for status in [400, 501, 505] {
            let error = ZaiError::from_api_response(status, 0, String::new());
            assert!(!error.is_retryable(), "HTTP {status} must not retry");
        }
        for status in [501, 505] {
            let error = ZaiError::from_api_response(status, 0, String::new());
            assert_eq!(error.category(), ErrorCategory::Other);
            assert!(!error.is_server_error());
        }
    }

    #[test]
    fn status_401_403_classify_as_auth() {
        for status in [401, 403] {
            let e = ZaiError::from_api_response(status, 0, String::new());
            assert!(
                e.is_auth_error(),
                "HTTP {status} should classify as auth, got {e:?}"
            );
            assert!(
                e.is_client_error(),
                "HTTP {status} should be a client error"
            );
            assert!(
                !e.is_retryable(),
                "HTTP {status} (auth) should not be retryable"
            );
        }
    }

    #[test]
    fn status_429_classifies_as_rate_limit() {
        let e = ZaiError::from_api_response(429, 0, String::new());
        assert!(
            e.is_rate_limit(),
            "HTTP 429 should classify as rate limit, got {e:?}"
        );
        assert!(e.is_retryable(), "HTTP 429 should be retryable");
    }
}
