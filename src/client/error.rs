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
//! | [`ZaiError::Request`] | — | HTTP-dispatched failure with structured request diagnostics |
//! | [`ZaiError::Context`] | — | Operational context around a source error that has no message slot |
//! | [`ZaiError::AuthError`] | 1000, 1001, 1003, 1005, 1220 | Authentication / authorization (invalid API key, etc.) |
//! | [`ZaiError::AccountError`] | 1110–1121 except 1113 | Account/package-related errors |
//! | [`ZaiError::ApiError`] | 1200–1234, 1261 | Request validation or upstream execution errors |
//! | [`ZaiError::ContentPolicyError`] | 1301 | API policy / unsafe-content blocks |
//! | [`ZaiError::RateLimitError`] | 1113, 1302, 1305, 1308–1311, 1313–1321 | Rate-limit, quota, package pressure or fair-use errors |
//! | [`ZaiError::FileError`] | 1400–1499 | File-processing errors |
//! | [`ZaiError::HttpBusinessError`] | — | Unrecognized business code; recovery falls back to the HTTP status |
//! | [`ZaiError::Unknown`] | other | Unrecognized business or HTTP errors |
//! | [`ZaiError::NetworkError`] | — | Network / timeout errors |
//! | [`ZaiError::JsonError`] | — | JSON serialization / deserialization errors |
//! | [`ZaiError::RealtimeError`] | — | Realtime WebSocket transport or protocol errors |
//! | [`ZaiError::RealtimeAuthError`] | — | Realtime API-key or JWT errors |
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
//! use zai_rs::client::error::ZaiResult;
//!
//! async fn call_api() -> ZaiResult<String> {
//!     Ok("result".to_string())
//! }
//!
//! # async fn example() {
//! match call_api().await {
//!     Ok(data) => println!("Success: {}", data),
//!     Err(error) if error.is_auth_error() => {
//!         tracing::error!(category = ?error.category(), "Authentication failed");
//!     },
//!     Err(error) if error.is_rate_limit() => {
//!         tracing::warn!(category = ?error.category(), "Request was rate limited");
//!     },
//!     Err(error) => tracing::error!(
//!         category = ?error.category(),
//!         retryable = error.is_retryable(),
//!         "API call failed",
//!     ),
//! }
//! # }
//! ```

use std::{fmt, sync::Arc, time::Duration};

use thiserror::Error;

/// Reserved error-code constants for failures originating inside the SDK.
pub mod codes;
mod redaction;

pub use redaction::{contains_sensitive_info, mask_api_key, mask_sensitive_info, validate_api_key};

/// Context for an unrecognized business code received with an actionable HTTP
/// error status.
///
/// The code is retained separately from the HTTP status so callers can
/// diagnose provider/proxy changes without allowing an unknown business code
/// to erase authentication, rate-limit, or server-failure classification.
/// [`Debug`] and [`Display`](fmt::Display) deliberately omit the business code;
/// callers must explicitly request its bounded, credential-redacted
/// representation through [`business_code`](Self::business_code).
#[derive(Clone)]
pub struct HttpBusinessErrorContext {
    status: u16,
    business_code: String,
    message: String,
}

impl HttpBusinessErrorContext {
    fn new(status: u16, business_code: String, message: String) -> Self {
        Self {
            status,
            business_code: mask_sensitive_info(&business_code),
            message: mask_sensitive_info(&message),
        }
    }

    /// HTTP response status used for classification and retry decisions.
    pub fn status(&self) -> u16 {
        self.status
    }

    /// Bounded canonical representation of the unrecognized wire code.
    ///
    /// Recognizable credentials are redacted before this value is stored.
    pub fn business_code(&self) -> &str {
        &self.business_code
    }

    /// Human-readable service error message with recognizable credentials
    /// redacted.
    ///
    /// Credential filtering is not arbitrary content redaction: provider text
    /// can still contain prompts, filenames, or other application data. Apply
    /// an application-specific content policy before logging this value.
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Debug for HttpBusinessErrorContext {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("HttpBusinessErrorContext")
            .field("status", &self.status)
            .field("business_code", &"[REDACTED]")
            .field("message", &self.message)
            .finish()
    }
}

impl fmt::Display for HttpBusinessErrorContext {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "HTTP error [{}] with unrecognized business code: {}",
            self.status, self.message
        )
    }
}

/// Phase in which an HTTP request deadline expired.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum TimeoutPhase {
    /// One ordinary HTTP attempt exceeded its deadline.
    Attempt,
    /// The deadline covering all attempts, redirects, and backoff expired.
    Overall,
    /// An SSE response was not established before its handshake deadline.
    SseHandshake,
    /// An established SSE response remained silent beyond its idle deadline.
    SseIdle,
    /// The request was not admitted before its concurrency-queue deadline.
    /// No HTTP attempt was made.
    Queue,
    /// An established indefinite raw response stream was not advanced before
    /// its consumer-progress deadline, so the entire stream was canceled.
    /// Finite file streams remain governed by the [`Overall`](Self::Overall)
    /// deadline and do not use this phase.
    StreamConsumer,
}

/// Bounded, structured diagnostics for a request dispatched by
/// [`ZaiClient`](crate::client::ZaiClient).
///
/// Metadata is attached once a request enters transport admission. A queue
/// timeout has `attempts() == 0`; local validation and serialization failures
/// occur earlier and therefore have no request metadata. A provider request
/// identifier is bounded and restricted to a conservative ASCII character set
/// before storage, but remains provider-controlled application data. It is
/// intentionally omitted from
/// [`Debug`] and from the parent error's [`fmt::Display`] output; callers must
/// explicitly read it through [`request_id`](Self::request_id) and apply their
/// own logging policy.
#[derive(Clone, Default, PartialEq, Eq)]
pub struct RequestErrorMetadata {
    request_id: Option<String>,
    attempts: u8,
    timeout_phase: Option<TimeoutPhase>,
    retry_after: Option<Duration>,
}

impl RequestErrorMetadata {
    pub(crate) fn for_attempts(attempts: u8) -> Self {
        Self {
            attempts,
            ..Self::default()
        }
    }

    pub(crate) fn with_request_id(mut self, request_id: Option<String>) -> Self {
        self.request_id = request_id;
        self
    }

    pub(crate) fn with_timeout_phase(mut self, timeout_phase: TimeoutPhase) -> Self {
        self.timeout_phase = Some(timeout_phase);
        self
    }

    pub(crate) fn with_retry_after(mut self, retry_after: Option<Duration>) -> Self {
        self.retry_after = retry_after;
        self
    }

    fn merge(self, newer: Self) -> Self {
        Self {
            request_id: newer.request_id.or(self.request_id),
            attempts: newer.attempts.max(self.attempts),
            timeout_phase: newer.timeout_phase.or(self.timeout_phase),
            retry_after: newer.retry_after.or(self.retry_after),
        }
    }

    /// Provider request identifier, when a bounded ASCII value was present.
    ///
    /// This explicit accessor returns the unredacted provider value. Treat it
    /// as application data rather than a generally safe logging field.
    pub fn request_id(&self) -> Option<&str> {
        self.request_id.as_deref()
    }

    /// Number of HTTP attempts made, including the initial attempt.
    pub const fn attempts(&self) -> u8 {
        self.attempts
    }

    /// Timeout phase, when the failure was caused by a deadline.
    pub const fn timeout_phase(&self) -> Option<TimeoutPhase> {
        self.timeout_phase
    }

    /// Final valid `Retry-After` hint observed from the service.
    pub const fn retry_after(&self) -> Option<Duration> {
        self.retry_after
    }
}

impl fmt::Debug for RequestErrorMetadata {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RequestErrorMetadata")
            .field("request_id_present", &self.request_id.is_some())
            .field("attempts", &self.attempts)
            .field("timeout_phase", &self.timeout_phase)
            .field("retry_after", &self.retry_after)
            .finish()
    }
}

/// Main error type for the ZAI-RS SDK
#[derive(Error, Debug, Clone)]
#[non_exhaustive]
pub enum ZaiError {
    /// A transport-dispatched request failure with structured diagnostics.
    ///
    /// Existing classification and message helpers transparently delegate to
    /// `source`. Use [`request_metadata`](Self::request_metadata) when the
    /// attempt count, timeout phase, request ID, or retry hint is needed.
    #[error("{source}")]
    Request {
        /// Original SDK, HTTP, or provider error.
        #[source]
        source: Arc<ZaiError>,
        /// Bounded transport diagnostics. The optional request identifier is
        /// provider-controlled and available only through an explicit accessor.
        metadata: RequestErrorMetadata,
    },

    /// Operational context retained around an error whose variant has no
    /// editable message slot.
    ///
    /// Classification, retry, code, and request-diagnostics helpers
    /// transparently delegate to `source`. Most callers should construct this
    /// through [`context`](Self::context), which redacts recognizable
    /// credentials before storing `context`.
    #[error("{context}: {source}")]
    #[non_exhaustive]
    Context {
        /// Original SDK, transport, or serialization error.
        #[source]
        source: Arc<ZaiError>,
        /// Credential-redacted operation description.
        context: String,
    },

    /// HTTP status errors
    #[error("HTTP error [{status}]: {message}")]
    HttpError {
        /// HTTP status code (e.g. `400`, `404`, `500`).
        status: u16,
        /// Human-readable error message returned with the response.
        message: String,
    },

    /// An unrecognized business code accompanied by an HTTP status whose
    /// authentication, rate-limit, or server semantics must remain visible.
    ///
    /// The HTTP status, rather than the unknown code, drives
    /// [`category`](Self::category) and [`is_retryable`](Self::is_retryable).
    #[error("{0}")]
    HttpBusinessError(HttpBusinessErrorContext),

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

/// Safe, structured context for an HTTP response that rejected a Realtime
/// WebSocket handshake.
///
/// The peer-controlled response headers and body are discarded before this
/// value enters the public error chain. Only the fields needed for diagnostics
/// and retry classification are retained.
#[cfg(feature = "realtime")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RealtimeHandshakeHttpContext {
    status: u16,
    business_code: Option<u16>,
    retry_after: Option<Duration>,
}

#[cfg(feature = "realtime")]
impl RealtimeHandshakeHttpContext {
    fn new(status: u16, business_code: Option<u16>, retry_after: Option<Duration>) -> Self {
        Self {
            status,
            business_code,
            retry_after,
        }
    }

    /// HTTP response status returned instead of a WebSocket upgrade.
    pub const fn status(&self) -> u16 {
        self.status
    }

    /// Canonical numeric provider business code recovered from a bounded body
    /// whose completeness was proven by unambiguous HTTP framing.
    pub const fn business_code(&self) -> Option<u16> {
        self.business_code
    }

    /// Valid `Retry-After` hint supplied by the peer, when present.
    pub const fn retry_after(&self) -> Option<Duration> {
        self.retry_after
    }
}

#[cfg(feature = "realtime")]
impl fmt::Display for RealtimeHandshakeHttpContext {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "HTTP error: {}", self.status)
    }
}

/// Concrete error categories for the realtime (WebSocket) transport.
///
/// Kept separate from [`ZaiError`] so callers can introspect the failure mode
/// without matching on the full enum, and so the realtime module can construct
/// rich errors without touching HTTP-specific machinery.
#[derive(thiserror::Error)]
#[non_exhaustive]
pub enum RealtimeErrorKind {
    /// Low-level non-HTTP WebSocket error (connect/read/write). The original
    /// `tungstenite` error is kept as the `#[source]` so the full chain
    /// survives propagation. HTTP handshake responses use
    /// [`HandshakeHttp`](Self::HandshakeHttp) instead. Only available with the
    /// `realtime` feature.
    #[cfg(feature = "realtime")]
    #[error("websocket: {source}")]
    WebSocket {
        /// The underlying tungstenite error.
        #[source]
        source: tokio_tungstenite::tungstenite::Error,
    },

    /// An HTTP response rejected the WebSocket handshake.
    ///
    /// Raw response headers and body bytes are deliberately omitted from this
    /// public error. The retained context is sufficient for classification and
    /// bounded connection-retry policy.
    #[cfg(feature = "realtime")]
    #[error("websocket handshake: {0}")]
    HandshakeHttp(RealtimeHandshakeHttpContext),

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

#[cfg(feature = "realtime")]
struct DisplayAsDebug<'a, T>(&'a T);

#[cfg(feature = "realtime")]
impl<T: fmt::Display> fmt::Debug for DisplayAsDebug<'_, T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self.0, formatter)
    }
}

impl fmt::Debug for RealtimeErrorKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            #[cfg(feature = "realtime")]
            Self::WebSocket { source } => formatter
                .debug_struct("WebSocket")
                .field("source", &DisplayAsDebug(source))
                .finish(),
            #[cfg(feature = "realtime")]
            Self::HandshakeHttp(context) => formatter
                .debug_tuple("HandshakeHttp")
                .field(context)
                .finish(),
            Self::Protocol(message) => formatter.debug_tuple("Protocol").field(message).finish(),
            Self::Timeout { operation } => formatter
                .debug_struct("Timeout")
                .field("operation", operation)
                .finish(),
            Self::Closed => formatter.write_str("Closed"),
        }
    }
}

/// Whether an I/O failure is a known transient WebSocket connection outcome.
///
/// This is shared by the public retry projection and the built-in Realtime
/// connection-attempt policy so their allowlists cannot drift apart.
#[cfg(feature = "realtime")]
pub(crate) const fn is_retryable_websocket_io(kind: std::io::ErrorKind) -> bool {
    use std::io::ErrorKind;

    matches!(
        kind,
        ErrorKind::ConnectionRefused
            | ErrorKind::ConnectionReset
            | ErrorKind::HostUnreachable
            | ErrorKind::NetworkUnreachable
            | ErrorKind::ConnectionAborted
            | ErrorKind::NotConnected
            | ErrorKind::NetworkDown
            | ErrorKind::BrokenPipe
            | ErrorKind::TimedOut
            | ErrorKind::Interrupted
            | ErrorKind::UnexpectedEof
    )
}

impl ZaiError {
    /// Convert an HTTP status code and API error response to a [`ZaiError`].
    pub fn from_api_response(status: u16, api_code: u16, api_message: String) -> Self {
        // Provider and proxy error bodies are untrusted and occasionally echo
        // request metadata. Remove recognizable credentials before the text
        // enters the public error value.
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
                _ if uses_http_fallback(status) => {
                    ZaiError::HttpBusinessError(HttpBusinessErrorContext::new(
                        status,
                        api_code.to_string(),
                        message_or(api_message, "Unknown error"),
                    ))
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

    pub(crate) fn from_unrecognized_business_response(
        status: u16,
        business_code: String,
        api_message: String,
    ) -> Self {
        if uses_http_fallback(status) {
            Self::HttpBusinessError(HttpBusinessErrorContext::new(
                status,
                business_code,
                message_or(api_message, "Unknown error"),
            ))
        } else {
            Self::from_api_response(status, 0, api_message)
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
            ZaiError::Request { source, .. } | ZaiError::Context { source, .. } => {
                source.category()
            },
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
                // non-HTTP WebSocket failures are network-level. A completed
                // or already-completed close is neither.
                RealtimeErrorKind::Protocol(_) => ErrorCategory::Client,
                #[cfg(feature = "realtime")]
                RealtimeErrorKind::WebSocket { source } => match source {
                    tokio_tungstenite::tungstenite::Error::ConnectionClosed
                    | tokio_tungstenite::tungstenite::Error::AlreadyClosed => ErrorCategory::Other,
                    _ => ErrorCategory::Network,
                },
                #[cfg(feature = "realtime")]
                RealtimeErrorKind::HandshakeHttp(context) => {
                    classify_realtime_handshake_http(context)
                },
                RealtimeErrorKind::Timeout { .. } => ErrorCategory::Network,
                RealtimeErrorKind::Closed => ErrorCategory::Other,
            },
            ZaiError::HttpError { status, .. } => classify_status(*status),
            ZaiError::HttpBusinessError(context) => classify_http_fallback(context.status),
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
            ZaiError::Request { source, .. } | ZaiError::Context { source, .. } => {
                source.is_retryable()
            },
            ZaiError::HttpError { status, .. } => {
                crate::client::transport::retry::RETRYABLE_STATUSES.contains(status)
            },
            ZaiError::HttpBusinessError(context) => {
                crate::client::transport::retry::RETRYABLE_STATUSES.contains(&context.status)
            },
            ZaiError::RateLimitError { code, .. } => matches!(*code, 429 | 1302 | 1305),
            ZaiError::NetworkError(_) => true,
            ZaiError::ApiError { code, .. } if *code == codes::SDK_TIMEOUT => true,
            ZaiError::ApiError { code, .. } if is_server_business_code(*code) => true,
            ZaiError::RealtimeError(kind) => match kind.as_ref() {
                RealtimeErrorKind::Timeout { .. } => true,
                #[cfg(feature = "realtime")]
                RealtimeErrorKind::HandshakeHttp(context) => {
                    is_retryable_realtime_handshake_http(context)
                },
                #[cfg(feature = "realtime")]
                RealtimeErrorKind::WebSocket {
                    source: tokio_tungstenite::tungstenite::Error::Io(error),
                } => is_retryable_websocket_io(error.kind()),
                _ => false,
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

    /// Return a compact error representation.
    ///
    /// Provider messages remain application data and can contain prompts,
    /// filenames, or other user content. Do not log this representation
    /// without an application-specific content policy.
    pub fn compact(&self) -> String {
        match self {
            ZaiError::Request { source, .. } => source.compact(),
            ZaiError::Context { source, context } => {
                let compact = source.compact();
                if let Some((category, detail)) = compact.split_once(": ") {
                    format!("{category}: {context}: {detail}")
                } else {
                    format!("{context}: {compact}")
                }
            },
            ZaiError::HttpError { status, message } => {
                format!("HTTP[{status}]: {message}")
            },
            ZaiError::HttpBusinessError(context) => {
                format!("HTTP[{}]: {}", context.status, context.message)
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
            ZaiError::Request { source, .. } | ZaiError::Context { source, .. } => source.code(),
            ZaiError::HttpError { status, .. } => Some(*status),
            ZaiError::HttpBusinessError(context) => Some(context.status),
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

    /// Return the unrecognized wire business code retained by an HTTP-status
    /// fallback error.
    ///
    /// The returned representation is bounded and canonicalized by the
    /// transport, with recognizable credentials redacted. Other error variants
    /// return `None`.
    pub fn raw_business_code(&self) -> Option<&str> {
        match self {
            Self::Request { source, .. } | Self::Context { source, .. } => {
                source.raw_business_code()
            },
            Self::HttpBusinessError(context) => Some(context.business_code()),
            _ => None,
        }
    }

    /// Structured diagnostics retained by the HTTP transport.
    ///
    /// Returns `None` for local failures that occurred before dispatch. The
    /// request ID is not rendered by `Display`, `Debug`, or [`compact`](Self::compact);
    /// reading this provider-controlled value is an explicit diagnostics
    /// action and requires an application-specific logging policy.
    pub fn request_metadata(&self) -> Option<&RequestErrorMetadata> {
        match self {
            Self::Request { metadata, .. } => Some(metadata),
            Self::Context { source, .. } => source.request_metadata(),
            _ => None,
        }
    }

    /// Original error beneath transparent request-diagnostics and operation-
    /// context wrappers.
    ///
    /// Returns `self` when neither wrapper is present.
    pub fn source_error(&self) -> &ZaiError {
        match self {
            Self::Request { source, .. } | Self::Context { source, .. } => source.source_error(),
            _ => self,
        }
    }

    pub(crate) fn with_request_metadata(self, metadata: RequestErrorMetadata) -> Self {
        match self {
            Self::Request {
                source,
                metadata: existing,
            } => Self::Request {
                source,
                metadata: existing.merge(metadata),
            },
            source => Self::Request {
                source: Arc::new(source),
                metadata,
            },
        }
    }

    /// Return the human-readable error message.
    ///
    /// Recognizable credentials are filtered on provider paths, but provider
    /// text can still contain prompts, filenames, or other application data.
    /// Apply an application-specific content policy before logging this value.
    pub fn message(&self) -> String {
        match self {
            ZaiError::Request { source, .. } => source.message(),
            ZaiError::Context { source, context } => {
                format!("{context}: {}", source.message())
            },
            ZaiError::HttpError { message, .. } => message.clone(),
            ZaiError::HttpBusinessError(context) => context.message.clone(),
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
    /// retain the operation through a transparent [`Context`](Self::Context)
    /// wrapper. The supplied context and editable variant messages are
    /// credential-redacted before storage; an already wrapped source error is
    /// retained byte-for-byte so its standard error chain remains available.
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
        let context = mask_sensitive_info(context);
        let with_context = |message: String| mask_sensitive_info(&format!("{context}: {message}"));
        match self {
            Self::Request { source, metadata } => Self::Request {
                source: Arc::new(source.as_ref().clone().context(&context)),
                metadata,
            },
            Self::Context {
                source,
                context: inner,
            } => Self::Context {
                source,
                context: with_context(inner),
            },
            Self::HttpError { status, message } => Self::HttpError {
                status,
                message: with_context(message),
            },
            Self::HttpBusinessError(mut error) => {
                error.message = with_context(error.message);
                Self::HttpBusinessError(error)
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
            source @ (Self::NetworkError(_) | Self::JsonError(_) | Self::RealtimeError(_)) => {
                Self::Context {
                    source: Arc::new(source),
                    context,
                }
            },
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

const fn uses_http_fallback(status: u16) -> bool {
    matches!(status, 401 | 403 | 429) || (status >= 500 && status < 600)
}

fn classify_http_fallback(status: u16) -> ErrorCategory {
    match status {
        401 | 403 => ErrorCategory::Auth,
        _ => classify_status(status),
    }
}

#[cfg(feature = "realtime")]
fn classify_realtime_handshake_http(context: &RealtimeHandshakeHttpContext) -> ErrorCategory {
    let Some(business_code) = context.business_code() else {
        return classify_http_fallback(context.status());
    };

    let mapped = ZaiError::from_api_response(context.status(), business_code, String::new());
    match mapped {
        // An unknown business code must not erase the HTTP status semantics.
        ZaiError::HttpBusinessError(_) | ZaiError::Unknown { .. } => {
            classify_http_fallback(context.status())
        },
        known => known.category(),
    }
}

#[cfg(feature = "realtime")]
fn is_retryable_realtime_handshake_http(context: &RealtimeHandshakeHttpContext) -> bool {
    crate::client::transport::retry::is_retryable_outcome(context.status(), context.business_code())
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
/// [`ZaiError`]. Non-HTTP errors are preserved as the `#[source]` of
/// [`RealtimeErrorKind::WebSocket`]. An HTTP handshake response is reduced to
/// [`RealtimeHandshakeHttpContext`] before entering the public error chain, so
/// peer-controlled headers and body bytes cannot leak through `Debug`.
/// Only available with the `realtime` feature.
#[cfg(feature = "realtime")]
impl From<tokio_tungstenite::tungstenite::Error> for ZaiError {
    fn from(err: tokio_tungstenite::tungstenite::Error) -> Self {
        use crate::client::transport::{
            business_code_from_complete_json, limits::ERROR_BODY_MAX, retry::parse_retry_after,
        };
        use tokio_tungstenite::tungstenite::Error as WebSocketError;

        let kind = match err {
            WebSocketError::Http(response) => {
                let status = response.status().as_u16();
                // Tungstenite stops reading as soon as it has parsed the HTTP
                // response headers. Its response body is only the tail that
                // happened to arrive in that same read, not necessarily the
                // complete HTTP entity. A partial JSON prefix must never
                // control retry or category. Trust it only when one
                // Content-Length proves exact completeness and no transfer
                // coding makes the framing ambiguous.
                let headers = response.headers();
                let mut content_lengths = headers.get_all(http::header::CONTENT_LENGTH).iter();
                let content_length = content_lengths
                    .next()
                    .and_then(|value| value.to_str().ok())
                    .and_then(|value| value.parse::<usize>().ok());
                let has_one_content_length = content_length.is_some()
                    && content_lengths.next().is_none()
                    && !headers.contains_key(http::header::TRANSFER_ENCODING);
                let business_code = response
                    .body()
                    .as_deref()
                    .filter(|body| {
                        has_one_content_length
                            && content_length == Some(body.len())
                            && body.len() <= ERROR_BODY_MAX as usize
                    })
                    .and_then(business_code_from_complete_json);
                let retry_after = response
                    .headers()
                    .get(http::header::RETRY_AFTER)
                    .and_then(|value| value.to_str().ok())
                    .and_then(parse_retry_after);
                RealtimeErrorKind::HandshakeHttp(RealtimeHandshakeHttpContext::new(
                    status,
                    business_code,
                    retry_after,
                ))
            },
            source => RealtimeErrorKind::WebSocket { source },
        };
        ZaiError::RealtimeError(Arc::new(kind))
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

    #[cfg(feature = "realtime")]
    fn handshake_http_error(status: u16, body: &[u8], headers: &[(&str, &str)]) -> ZaiError {
        use tokio_tungstenite::tungstenite::Error as WebSocketError;

        let mut response = http::Response::builder()
            .status(status)
            .header(http::header::CONTENT_LENGTH, body.len().to_string());
        for (name, value) in headers {
            response = response.header(*name, *value);
        }
        WebSocketError::Http(Box::new(response.body(Some(body.to_vec())).unwrap())).into()
    }

    #[cfg(feature = "realtime")]
    fn unframed_handshake_http_error(
        status: u16,
        body: &[u8],
        headers: &[(&str, &str)],
    ) -> ZaiError {
        use tokio_tungstenite::tungstenite::Error as WebSocketError;

        let mut response = http::Response::builder().status(status);
        for (name, value) in headers {
            response = response.header(*name, *value);
        }
        WebSocketError::Http(Box::new(response.body(Some(body.to_vec())).unwrap())).into()
    }

    #[cfg(feature = "realtime")]
    fn handshake_context(error: &ZaiError) -> &RealtimeHandshakeHttpContext {
        let ZaiError::RealtimeError(kind) = error.source_error() else {
            panic!("handshake failure must remain a realtime error");
        };
        let RealtimeErrorKind::HandshakeHttp(context) = kind.as_ref() else {
            panic!("raw HTTP response entered the public error chain");
        };
        context
    }

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

    #[cfg(feature = "realtime")]
    #[test]
    fn realtime_handshake_http_discards_raw_response_from_every_default_rendering() {
        let header_secret = "header-secret-value";
        let body_secret = br#"{"code":1001,"message":"body-secret-value"}"#;
        let body_debug = format!("{body_secret:?}");
        let error = handshake_http_error(
            401,
            body_secret,
            &[
                ("set-cookie", "session=private-cookie"),
                ("www-authenticate", header_secret),
                ("x-private-diagnostic", "private-diagnostic-value"),
                ("retry-after", "7"),
            ],
        );

        let context = handshake_context(&error);
        assert_eq!(context.status(), 401);
        assert_eq!(context.business_code(), Some(1001));
        assert_eq!(context.retry_after(), Some(Duration::from_secs(7)));
        let cloned = RealtimeHandshakeHttpContext::clone(context);
        assert_eq!(&cloned, context);

        let contextual = error.clone().context("open realtime session");
        let requested = error.with_request_metadata(RequestErrorMetadata::for_attempts(2));
        for rendered in [
            contextual.to_string(),
            contextual.message(),
            contextual.compact(),
            format!("{contextual:?}"),
            format!("{contextual:#?}"),
            format!("{requested:?}"),
            format!("{requested:#?}"),
        ] {
            for forbidden in [
                header_secret,
                "private-cookie",
                "private-diagnostic-value",
                "x-private-diagnostic",
                "body-secret-value",
                body_debug.as_str(),
            ] {
                assert!(
                    !rendered.contains(forbidden),
                    "handshake response leaked through `{rendered}`"
                );
            }
        }

        let ZaiError::RealtimeError(kind) = requested.source_error() else {
            panic!("request wrapper must retain the realtime error");
        };
        assert!(matches!(kind.as_ref(), RealtimeErrorKind::HandshakeHttp(_)));
        assert!(
            std::error::Error::source(kind.as_ref()).is_none(),
            "raw tungstenite HTTP response remained in the source chain"
        );
    }

    #[cfg(feature = "realtime")]
    #[test]
    fn realtime_handshake_summary_uses_only_bounded_canonical_inputs() {
        use crate::client::transport::limits::ERROR_BODY_MAX;

        let mut body = vec![b' '; ERROR_BODY_MAX as usize];
        body.extend_from_slice(br#"{"code":1302}"#);
        let error = handshake_http_error(503, &body, &[("retry-after", "invalid-retry-secret")]);
        let context = handshake_context(&error);

        assert_eq!(context.status(), 503);
        assert_eq!(
            context.business_code(),
            None,
            "a code beyond the diagnostic cap must not affect policy"
        );
        assert_eq!(context.retry_after(), None);
        assert!(
            error.is_retryable(),
            "status-only 503 should remain transient"
        );
        assert!(!format!("{error:#?}").contains("invalid-retry-secret"));
    }

    #[cfg(feature = "realtime")]
    #[test]
    fn realtime_handshake_rejects_ambiguous_or_malformed_business_codes() {
        for body in [
            br#"{"code":1302,"code":1113}"# as &[u8],
            br#"{"code":1302,"code":1113"#,
            br#"{"error":{"code":1113,"code":1302}}"#,
        ] {
            let error = handshake_http_error(400, body, &[]);
            let context = handshake_context(&error);
            assert_eq!(context.business_code(), None);
            assert_eq!(error.category(), ErrorCategory::Client);
            assert!(!error.is_retryable());
        }
    }

    #[cfg(feature = "realtime")]
    #[test]
    fn realtime_handshake_business_code_requires_complete_unambiguous_framing() {
        let body = br#"{"code":1302}"#;
        let cases: &[(&[(&str, &str)], &str)] = &[
            (&[], "missing Content-Length"),
            (&[("content-length", "1")], "mismatched Content-Length"),
            (
                &[("content-length", "13"), ("content-length", "13")],
                "duplicate Content-Length",
            ),
            (
                &[("content-length", "13"), ("transfer-encoding", "chunked")],
                "transfer coding",
            ),
        ];

        for (headers, reason) in cases {
            let error = unframed_handshake_http_error(400, body, headers);
            assert_eq!(
                handshake_context(&error).business_code(),
                None,
                "{reason} must not make a Tungstenite tail authoritative"
            );
            assert_eq!(error.category(), ErrorCategory::Client);
            assert!(!error.is_retryable());
        }

        let complete = handshake_http_error(400, body, &[]);
        assert_eq!(handshake_context(&complete).business_code(), Some(1302));
        assert_eq!(complete.category(), ErrorCategory::RateLimit);
        assert!(complete.is_retryable());
    }

    #[cfg(feature = "realtime")]
    #[test]
    fn realtime_websocket_debug_uses_display_even_for_manually_wrapped_payloads() {
        use tokio_tungstenite::tungstenite::{Error as WebSocketError, Message};

        let header_secret = "manually-wrapped-header-secret";
        let body_secret = b"manually-wrapped-body-secret";
        let response = http::Response::builder()
            .status(418)
            .header("x-private-response", header_secret)
            .body(Some(body_secret.to_vec()))
            .unwrap();
        let raw_http = ZaiError::from(RealtimeErrorKind::WebSocket {
            source: WebSocketError::Http(Box::new(response)),
        })
        .context("manual transport");
        let raw_http_debug = format!("{raw_http:#?}");
        assert!(!raw_http_debug.contains(header_secret));
        assert!(!raw_http_debug.contains("x-private-response"));
        assert!(!raw_http_debug.contains(&format!("{body_secret:?}")));
        assert!(raw_http_debug.contains("HTTP error: 418"));

        let message_secret = "private outbound message";
        let write_buffer = ZaiError::from(RealtimeErrorKind::WebSocket {
            source: WebSocketError::WriteBufferFull(Box::new(Message::Text(message_secret.into()))),
        });
        let write_buffer_debug = format!("{write_buffer:#?}");
        assert!(!write_buffer_debug.contains(message_secret));
        assert!(write_buffer_debug.contains("Write buffer is full"));
    }

    #[cfg(feature = "realtime")]
    #[test]
    fn realtime_handshake_http_category_and_retry_match_http_business_policy() {
        let cases: &[(u16, &[u8], ErrorCategory, bool)] = &[
            (401, br#"{}"#, ErrorCategory::Auth, false),
            (400, br#"{}"#, ErrorCategory::Client, false),
            (429, br#"{}"#, ErrorCategory::RateLimit, true),
            (
                429,
                br#"{"error":{"code":1113}}"#,
                ErrorCategory::RateLimit,
                false,
            ),
            (503, br#"{}"#, ErrorCategory::Server, true),
            (503, br#"{"code":1210}"#, ErrorCategory::Client, false),
            (400, br#"{"code":1302}"#, ErrorCategory::RateLimit, true),
            (400, br#"{"code":65000}"#, ErrorCategory::Client, false),
            (501, br#"{}"#, ErrorCategory::Other, false),
        ];

        for (status, body, category, retryable) in cases {
            let error = handshake_http_error(*status, body, &[]);
            assert_eq!(
                error.category(),
                *category,
                "wrong category for HTTP {status} body {}",
                String::from_utf8_lossy(body)
            );
            assert_eq!(
                error.is_retryable(),
                *retryable,
                "wrong retry decision for HTTP {status} body {}",
                String::from_utf8_lossy(body)
            );
            let contextual = error.context("connect");
            assert_eq!(contextual.category(), *category);
            assert_eq!(contextual.is_retryable(), *retryable);
        }
    }

    #[cfg(feature = "realtime")]
    #[test]
    fn realtime_non_http_websocket_retry_is_an_explicit_io_allowlist() {
        use tokio_tungstenite::tungstenite::{
            Error as WebSocketError,
            error::{CapacityError, ProtocolError, TlsError, UrlError},
        };

        for kind in [
            ErrorKind::ConnectionRefused,
            ErrorKind::ConnectionReset,
            ErrorKind::HostUnreachable,
            ErrorKind::NetworkUnreachable,
            ErrorKind::ConnectionAborted,
            ErrorKind::NotConnected,
            ErrorKind::NetworkDown,
            ErrorKind::BrokenPipe,
            ErrorKind::TimedOut,
            ErrorKind::Interrupted,
            ErrorKind::UnexpectedEof,
        ] {
            assert!(is_retryable_websocket_io(kind));
            let error = ZaiError::from(WebSocketError::Io(std::io::Error::new(kind, "test")));
            assert_eq!(error.category(), ErrorCategory::Network);
            assert!(error.is_retryable(), "{kind:?} must remain retryable");
        }

        for kind in [
            ErrorKind::InvalidData,
            ErrorKind::InvalidInput,
            ErrorKind::PermissionDenied,
            ErrorKind::NotFound,
            ErrorKind::AddrInUse,
            ErrorKind::AddrNotAvailable,
            ErrorKind::WouldBlock,
            ErrorKind::OutOfMemory,
            ErrorKind::Other,
        ] {
            assert!(!is_retryable_websocket_io(kind));
            let error = ZaiError::from(WebSocketError::Io(std::io::Error::new(kind, "test")));
            assert_eq!(error.category(), ErrorCategory::Network);
            assert!(!error.is_retryable(), "{kind:?} must fail closed");
        }

        let permanent = [
            WebSocketError::Protocol(ProtocolError::HandshakeIncomplete),
            WebSocketError::Url(UrlError::UnsupportedUrlScheme),
            WebSocketError::Tls(TlsError::InvalidDnsName),
            WebSocketError::Capacity(CapacityError::MessageTooLong {
                size: 2,
                max_size: 1,
            }),
            WebSocketError::AttackAttempt,
            WebSocketError::Utf8("invalid text".to_owned()),
        ];
        for source in permanent {
            let error = ZaiError::from(source);
            assert_eq!(error.category(), ErrorCategory::Network);
            assert!(!error.is_retryable());
        }

        for source in [
            WebSocketError::ConnectionClosed,
            WebSocketError::AlreadyClosed,
        ] {
            let error = ZaiError::from(source);
            assert_eq!(error.category(), ErrorCategory::Other);
            assert!(!error.is_retryable());
        }
    }

    #[test]
    fn context_retains_source_only_error_details_and_semantics() {
        let json_source = serde_json::from_str::<serde_json::Value>("{")
            .expect_err("the incomplete object must fail");
        let json_error = ZaiError::from(json_source).context("decode chat response");

        assert!(matches!(
            &json_error,
            ZaiError::Context { source, context }
                if matches!(source.as_ref(), ZaiError::JsonError(_))
                    && context == "decode chat response"
        ));
        assert_eq!(json_error.category(), ErrorCategory::Serialization);
        assert_eq!(json_error.code(), None);
        assert!(!json_error.is_retryable());
        assert!(json_error.message().starts_with("decode chat response: "));
        assert!(
            json_error
                .compact()
                .starts_with("JSON: decode chat response: ")
        );
        assert!(
            json_error
                .to_string()
                .starts_with("decode chat response: JSON error: ")
        );
        assert!(matches!(json_error.source_error(), ZaiError::JsonError(_)));
        let error_source = std::error::Error::source(&json_error)
            .expect("the context wrapper must retain a standard error source");
        assert!(error_source.to_string().starts_with("JSON error: "));
        assert!(error_source.source().is_some());

        let realtime_error = ZaiError::from(RealtimeErrorKind::Timeout { operation: "write" })
            .context("send session update");
        assert_eq!(realtime_error.category(), ErrorCategory::Network);
        assert!(realtime_error.is_retryable());
        assert_eq!(realtime_error.code(), None);
        assert_eq!(
            realtime_error.message(),
            "send session update: write timed out"
        );
        assert!(matches!(
            realtime_error.source_error(),
            ZaiError::RealtimeError(_)
        ));
        let realtime_leaf = std::error::Error::source(&realtime_error)
            .expect("context must link to the realtime SDK error");
        assert_eq!(realtime_leaf.to_string(), "Realtime error: write timed out");
        assert_eq!(
            realtime_leaf
                .source()
                .expect("realtime SDK error must link to its concrete kind")
                .to_string(),
            "write timed out"
        );

        let protocol_error = ZaiError::from(RealtimeErrorKind::Protocol("bad frame".to_owned()))
            .context("decode server event");
        assert_eq!(protocol_error.category(), ErrorCategory::Client);
        assert!(!protocol_error.is_retryable());
        assert!(matches!(
            protocol_error.source_error(),
            ZaiError::RealtimeError(_)
        ));

        let closed_error = ZaiError::from(RealtimeErrorKind::Closed).context("observe session");
        assert_eq!(closed_error.category(), ErrorCategory::Other);
        assert!(!closed_error.is_retryable());
        assert_eq!(closed_error.message(), "observe session: session closed");

        #[cfg(feature = "realtime")]
        {
            let websocket_error =
                ZaiError::from(tokio_tungstenite::tungstenite::Error::ConnectionClosed)
                    .context("read websocket frame");
            assert_eq!(websocket_error.category(), ErrorCategory::Other);
            assert!(!websocket_error.is_retryable());
            assert!(matches!(
                websocket_error.source_error(),
                ZaiError::RealtimeError(_)
            ));
            assert!(
                std::error::Error::source(&websocket_error)
                    .and_then(std::error::Error::source)
                    .and_then(std::error::Error::source)
                    .is_some()
            );
        }

        let network_source = reqwest::Client::new()
            .get("not a valid absolute URL")
            .build()
            .expect_err("a relative URL must fail request construction");
        let network_error = ZaiError::from(network_source).context("list models");
        assert_eq!(network_error.category(), ErrorCategory::Network);
        assert!(network_error.is_retryable());
        assert!(network_error.message().starts_with("list models: "));
        assert!(matches!(
            network_error.source_error(),
            ZaiError::NetworkError(_)
        ));
        assert!(
            std::error::Error::source(&network_error)
                .and_then(std::error::Error::source)
                .is_some()
        );
    }

    #[test]
    fn source_only_context_composes_with_metadata_and_redaction() {
        let secret = "abc123.abcdefghijklmnopqrstuvwxyz";
        let source = serde_json::from_str::<serde_json::Value>("{")
            .expect_err("the incomplete object must fail");
        let contextual = ZaiError::from(source)
            .context(&format!("decode Authorization: Bearer {secret}"))
            .context("agent invoke");
        let ZaiError::Context {
            source: stored_source,
            context: stored_context,
            ..
        } = &contextual
        else {
            panic!("source-only errors must use one context wrapper");
        };
        assert!(matches!(stored_source.as_ref(), ZaiError::JsonError(_)));
        assert!(stored_context.starts_with("agent invoke: "));
        assert!(stored_context.contains("decode"));
        assert!(stored_context.contains("[AUTH_REDACTED]"));
        assert!(!stored_context.contains(secret));
        let error = contextual.with_request_metadata(RequestErrorMetadata::for_attempts(2));

        assert_eq!(error.category(), ErrorCategory::Serialization);
        assert!(!error.is_retryable());
        assert_eq!(error.request_metadata().unwrap().attempts(), 2);
        assert!(matches!(error.source_error(), ZaiError::JsonError(_)));
        for rendered in [
            error.to_string(),
            format!("{error:?}"),
            error.compact(),
            error.message(),
        ] {
            assert!(rendered.contains("agent invoke"));
            assert!(rendered.contains("decode"));
            assert!(rendered.contains("[AUTH_REDACTED]"));
            assert!(!rendered.contains(secret));
            assert!(!rendered.contains("abcdefghijklmnopqrstuvwxyz"));
        }
    }

    #[test]
    fn request_metadata_and_source_context_compose_in_either_order() {
        let source = ZaiError::from(
            serde_json::from_str::<serde_json::Value>("{")
                .expect_err("the incomplete object must fail"),
        );
        let metadata =
            RequestErrorMetadata::for_attempts(3).with_request_id(Some("request-42".to_owned()));

        let context_then_metadata = source
            .clone()
            .context("decode response")
            .with_request_metadata(metadata.clone());
        let metadata_then_context = source
            .with_request_metadata(metadata)
            .context("decode response");

        for error in [&context_then_metadata, &metadata_then_context] {
            assert_eq!(error.category(), ErrorCategory::Serialization);
            assert_eq!(error.request_metadata().unwrap().attempts(), 3);
            assert_eq!(
                error.request_metadata().unwrap().request_id(),
                Some("request-42")
            );
            assert_eq!(error.message(), context_then_metadata.message());
            assert_eq!(error.compact(), context_then_metadata.compact());
            assert!(matches!(error.source_error(), ZaiError::JsonError(_)));
            assert!(!error.to_string().contains("request-42"));
            assert!(!format!("{error:?}").contains("request-42"));
        }

        assert!(matches!(
            &context_then_metadata,
            ZaiError::Request { source, .. }
                if matches!(source.as_ref(), ZaiError::Context { source, .. }
                    if matches!(source.as_ref(), ZaiError::JsonError(_)))
        ));
        assert!(matches!(
            &metadata_then_context,
            ZaiError::Request { source, .. }
                if matches!(source.as_ref(), ZaiError::Context { source, .. }
                    if matches!(source.as_ref(), ZaiError::JsonError(_)))
        ));
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
    fn quoted_json_credentials_are_fully_redacted() {
        let cases = [
            (r#"{"token":"opaque-secret"}"#, "opaque-secret"),
            (r#"{"password": "two words, still secret"}"#, "two words"),
            (r#"{"api_key":"opaque\\\"suffix"}"#, "suffix"),
            (
                r#"{"authorization":"Bearer opaque-token-value"}"#,
                "opaque-token-value",
            ),
            (
                r#"{'token':'single-quoted-secret'}"#,
                "single-quoted-secret",
            ),
            (r#"{"token":truncated-secret"#, "truncated-secret"),
            (
                r#"{"token":"truncated-quoted-secret"#,
                "truncated-quoted-secret",
            ),
            (
                r#"{"token":"prefix\"escaped-quote-secret"#,
                "escaped-quote-secret",
            ),
            (
                r#"{"token":"trailing-backslash-secret\"#,
                "backslash-secret",
            ),
            (
                r#"{'password':'prefix\'single-quote-secret"#,
                "single-quote-secret",
            ),
            (r#"{"authorization":Bearer opaque-token}"#, "opaque-token"),
            (r#"{"password":two words still-secret}"#, "still-secret"),
            (r#"{"token":{"raw":"object-secret"}}"#, "object-secret"),
            (r#"{"token":["array-secret"]}"#, "array-secret"),
            (
                r#"{"to\u006ben":"escaped-key-secret"}"#,
                "escaped-key-secret",
            ),
            (r#"{"api\u005fkey":"escaped-api-key"}"#, "escaped-api-key"),
            (r#"{"access_token":"opaque-access"}"#, "opaque-access"),
            (r#"{"client_secret":"opaque-client"}"#, "opaque-client"),
        ];

        for (input, forbidden) in cases {
            assert!(
                contains_sensitive_info(input),
                "missed JSON credential: {input}"
            );
            let filtered = mask_sensitive_info(input);
            assert!(filtered.contains("[FILTERED]"));
            assert!(
                !filtered.contains(forbidden),
                "credential suffix survived redaction: {filtered}"
            );
        }
    }

    #[test]
    fn structured_json_redacts_every_sensitive_key_and_embedded_credential() {
        let input = r#"{
            "token":"primary-secret",
            "debug":"abc.abcdefghijklmnop",
            "message":"Bearer secondary-secret",
            "items":[
                {"access_token":"access-secret"},
                {"client_secret":{"raw":"nested-secret"}}
            ]
        }"#;

        let filtered = mask_sensitive_info(input);
        serde_json::from_str::<serde_json::Value>(&filtered)
            .expect("redacted structured JSON must remain valid JSON");
        for forbidden in [
            "primary-secret",
            "abcdefghijklmnop",
            "secondary-secret",
            "access-secret",
            "nested-secret",
        ] {
            assert!(
                !filtered.contains(forbidden),
                "credential survived structured redaction: {filtered}"
            );
        }
        assert!(filtered.matches("[FILTERED]").count() >= 5);
    }

    #[test]
    fn contains_and_mask_agree_on_decoded_and_double_encoded_credentials() {
        let cases = [
            (
                r#"{"message":"to\u006ben\u003dencoded-secret"}"#,
                "encoded-secret",
            ),
            (
                r#"{"message":"Bearer\u0020encoded-bearer"}"#,
                "encoded-bearer",
            ),
            (
                r#"{"message":"abc\u002eabcdefghijklmnop"}"#,
                "abcdefghijklmnop",
            ),
            (
                r#"{"message":"{\"token\":\"double-secret\"}"}"#,
                "double-secret",
            ),
            (
                r#"{"message":"{\"to\\u006ben\":\"double-unicode-secret\"}"}"#,
                "double-unicode-secret",
            ),
            (
                r#"proxy={\"token\":\"direct-double-secret\"}"#,
                "direct-double-secret",
            ),
        ];

        for (input, forbidden) in cases {
            assert!(
                contains_sensitive_info(input),
                "contains/mask gate missed: {input}"
            );
            let filtered = mask_sensitive_info(input);
            assert!(
                !filtered.contains(forbidden),
                "credential survived: {filtered}"
            );
        }
    }

    #[test]
    fn structured_json_redacts_compound_fields_and_credential_keys() {
        let input = r#"{
            "db_password":"db-secret",
            "userPassword":"user-secret",
            "secret_key":"secret-key-value",
            "api_secret_key":"api-secret-key-value",
            "abc.abcdefghijklmnop":"value",
            "Bearer opaque-secondary":"value",
            "token":"primary"
        }"#;
        assert!(contains_sensitive_info(input));
        let filtered = mask_sensitive_info(input);
        serde_json::from_str::<serde_json::Value>(&filtered)
            .expect("redacted structured JSON must remain valid JSON");
        for forbidden in [
            "db-secret",
            "user-secret",
            "secret-key-value",
            "api-secret-key-value",
            "abcdefghijklmnop",
            "opaque-secondary",
            "primary",
        ] {
            assert!(
                !filtered.contains(forbidden),
                "credential survived: {filtered}"
            );
        }
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

    #[test]
    fn unknown_numeric_business_codes_fall_back_to_recovery_http_statuses() {
        for (status, category, retryable) in [
            (401, ErrorCategory::Auth, false),
            (403, ErrorCategory::Auth, false),
            (429, ErrorCategory::RateLimit, true),
            (503, ErrorCategory::Server, true),
        ] {
            let error = ZaiError::from_api_response(status, 7777, "provider changed".to_string());
            assert!(
                matches!(&error, ZaiError::HttpBusinessError(_)),
                "HTTP {status} should retain its recovery semantics: {error:?}"
            );
            assert_eq!(error.code(), Some(status));
            assert_eq!(error.raw_business_code(), Some("7777"));
            assert_eq!(error.category(), category);
            assert_eq!(error.is_retryable(), retryable);
        }
    }

    #[test]
    fn known_business_code_still_takes_precedence_over_http_status() {
        let error = ZaiError::from_api_response(503, 1210, "invalid parameters".to_string());
        assert!(matches!(&error, ZaiError::ApiError { code: 1210, .. }));
        assert_eq!(error.category(), ErrorCategory::Client);
        assert!(!error.is_retryable());
        assert_eq!(error.raw_business_code(), None);
    }

    #[test]
    fn unrecognized_business_code_is_hidden_from_default_rendering() {
        let secret = "api_key=abc123.abcdefghijklmnopqrstuvwxyz";
        let error = ZaiError::from_unrecognized_business_response(
            503,
            secret.to_string(),
            "upstream failed".to_string(),
        );

        let display = error.to_string();
        let debug = format!("{error:?}");
        let compact = error.compact();
        for rendered in [&display, &debug, &compact] {
            assert!(!rendered.contains(secret));
            assert!(!rendered.contains("abcdefghijklmnopqrstuvwxyz"));
        }
        let diagnostic = error.raw_business_code().unwrap();
        assert!(!diagnostic.contains("abcdefghijklmnopqrstuvwxyz"));
        assert!(diagnostic.contains("[FILTERED]"));
    }

    #[test]
    fn context_cannot_reintroduce_credentials_into_public_errors() {
        let secret = "abc123.abcdefghijklmnopqrstuvwxyz";
        let error = ZaiError::from_unrecognized_business_response(
            503,
            "UPSTREAM_BUSY".to_string(),
            "upstream failed".to_string(),
        )
        .context(&format!("Authorization: Bearer {secret}"));

        for rendered in [
            error.to_string(),
            format!("{error:?}"),
            error.compact(),
            error.message(),
        ] {
            assert!(!rendered.contains(secret));
            assert!(!rendered.contains("abcdefghijklmnopqrstuvwxyz"));
            assert!(rendered.contains("[AUTH_REDACTED]"));
        }
    }

    #[test]
    fn request_metadata_merges_and_delegates_without_leaking_request_ids() {
        let error = ZaiError::from_unrecognized_business_response(
            503,
            "UPSTREAM_BUSY".to_string(),
            "upstream failed".to_string(),
        )
        .with_request_metadata(
            RequestErrorMetadata::for_attempts(2)
                .with_request_id(Some("request-42".to_string()))
                .with_timeout_phase(TimeoutPhase::Attempt)
                .with_retry_after(Some(Duration::from_secs(3))),
        )
        .with_request_metadata(
            RequestErrorMetadata::for_attempts(1).with_timeout_phase(TimeoutPhase::Overall),
        )
        .with_request_metadata(
            RequestErrorMetadata::for_attempts(4)
                .with_request_id(Some("request-99".to_string()))
                .with_retry_after(Some(Duration::from_secs(5))),
        );

        let metadata = error.request_metadata().unwrap();
        assert_eq!(metadata.request_id(), Some("request-99"));
        assert_eq!(metadata.attempts(), 4);
        assert_eq!(metadata.timeout_phase(), Some(TimeoutPhase::Overall));
        assert_eq!(metadata.retry_after(), Some(Duration::from_secs(5)));
        assert_eq!(error.category(), ErrorCategory::Server);
        assert!(error.is_retryable());
        assert_eq!(error.code(), Some(503));
        assert_eq!(error.raw_business_code(), Some("UPSTREAM_BUSY"));
        assert_eq!(error.message(), "upstream failed");
        assert_eq!(error.compact(), "HTTP[503]: upstream failed");
        assert!(matches!(
            error.source_error(),
            ZaiError::HttpBusinessError(_)
        ));

        let metadata_debug = format!("{metadata:?}");
        assert!(metadata_debug.contains("request_id_present: true"));
        for rendered in [error.to_string(), format!("{error:?}"), error.compact()] {
            assert!(!rendered.contains("request-42"));
            assert!(!rendered.contains("request-99"));
        }

        let contextual = error.context("file list");
        assert_eq!(contextual.message(), "file list: upstream failed");
        assert_eq!(
            contextual.request_metadata().unwrap().request_id(),
            Some("request-99")
        );

        let local = ZaiError::ApiError {
            code: 1200,
            message: "local validation".to_string(),
        };
        assert_eq!(local.request_metadata(), None);
        assert!(matches!(local.source_error(), ZaiError::ApiError { .. }));
    }

    #[test]
    fn stream_consumer_timeout_phase_is_preserved_in_safe_metadata() {
        let metadata =
            RequestErrorMetadata::for_attempts(1).with_timeout_phase(TimeoutPhase::StreamConsumer);

        assert_eq!(metadata.attempts(), 1);
        assert_eq!(metadata.timeout_phase(), Some(TimeoutPhase::StreamConsumer));
        assert!(format!("{metadata:?}").contains("StreamConsumer"));
    }

    #[test]
    fn timeout_phase_addition_preserves_existing_discriminants() {
        assert_eq!(TimeoutPhase::Attempt as u8, 0);
        assert_eq!(TimeoutPhase::Overall as u8, 1);
        assert_eq!(TimeoutPhase::SseHandshake as u8, 2);
        assert_eq!(TimeoutPhase::SseIdle as u8, 3);
        assert_eq!(TimeoutPhase::Queue as u8, 4);
        assert_eq!(TimeoutPhase::StreamConsumer as u8, 5);
    }
}
