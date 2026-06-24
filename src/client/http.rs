//! # HTTP Client Implementation
//!
//! Provides a robust HTTP client for communicating with the Zhipu AI API.
//! This module implements connection pooling, error handling, and
//! request/response processing.
//!
//! ## Features
//!
//! - Connection Pooling - Reuses HTTP connections for better performance
//! - Error Handling - Comprehensive error parsing and reporting
//! - Authentication - Bearer token authentication support
//! - Retry with Jitter - Automatic retry with exponential backoff and random
//!   jitter
//! - Sensitive Data Masking - Automatic masking of API keys in logs
//! - Structured Logging - Uses tracing for detailed request/response logging
//!
//! ## Usage
//!
//! The `HttpClient` trait provides a standardized interface for making HTTP
//! requests to the Zhipu AI API endpoints.
//!
//! # Retry Configuration
//!
//! The HTTP client supports configurable retry behavior:
//!
//! ```ignore
//! use zai_rs::client::http::HttpClientConfig;
//!
//! let config = HttpClientConfig::builder()
//!     .max_retries(5)
//!     .timeout(Duration::from_secs(120))
//!     .retry_delay(RetryDelay::exponential(Duration::from_millis(100), Duration::from_secs(10)))
//!     .build();
//! ```

use std::{
    sync::{Arc, OnceLock},
    time::Duration,
};

use bytes::Bytes;
use reqwest::Method;
use serde::{Deserialize, de::DeserializeOwned};
use tracing::{trace, warn};

use crate::client::error::{ZaiError, ZaiResult, mask_sensitive_info};

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum ApiErrorEnvelope {
    Nested { error: ApiError },
    Flat { code: ErrorCode, message: String },
}

impl ApiErrorEnvelope {
    fn into_parts(self) -> (ErrorCode, String) {
        match self {
            ApiErrorEnvelope::Nested { error } => (error.code, error.message),
            ApiErrorEnvelope::Flat { code, message } => (code, message),
        }
    }
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

/// Parse an API error response body into a ZaiError.
///
/// Attempts to deserialize the body as `{"error":{"code":...,"message":...}}`
/// and maps it to the appropriate ZaiError variant. Falls back to a generic
/// HttpError if parsing fails.
pub fn parse_api_error_response(status: u16, body: String) -> crate::client::error::ZaiError {
    if let Ok(parsed) = serde_json::from_str::<ApiErrorEnvelope>(&body) {
        let (code, message) = parsed.into_parts();
        let api_code = to_api_code(&code);
        crate::client::error::ZaiError::from_api_response(status, api_code, message)
    } else {
        crate::client::error::ZaiError::from_api_response(status, 0, body)
    }
}

/// Retry delay strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryDelay {
    /// Fixed delay between retries
    Fixed(Duration),

    /// Exponential backoff with jitter
    Exponential {
        /// Initial (base) delay between retries; doubled on each attempt.
        base: Duration,
        /// Upper bound for the backoff delay.
        max: Duration,
    },

    /// No delay (not recommended for production)
    None,
}

impl RetryDelay {
    /// Create a fixed delay strategy
    pub fn fixed(delay: Duration) -> Self {
        Self::Fixed(delay)
    }

    /// Create an exponential backoff strategy
    pub fn exponential(base: Duration, max: Duration) -> Self {
        Self::Exponential { base, max }
    }

    /// Create a no-delay strategy (not recommended)
    pub fn none() -> Self {
        Self::None
    }
}

impl Default for RetryDelay {
    fn default() -> Self {
        Self::Exponential {
            base: Duration::from_millis(500),
            max: Duration::from_secs(5),
        }
    }
}

/// Configuration for HTTP client behavior.
///
/// Use the builder pattern for fluent configuration:
///
/// ```ignore
/// use zai_rs::client::http::HttpClientConfig;
///
/// let config = HttpClientConfig::builder()
///     .max_retries(5)
///     .timeout(Duration::from_secs(120))
///     .retry_delay(RetryDelay::exponential(Duration::from_millis(100), Duration::from_secs(10)))
///     .enable_logging(true)
///     .build();
/// ```
#[derive(Debug, Clone)]
pub struct HttpClientConfig {
    /// Request timeout duration (default: 60 seconds)
    pub timeout: Duration,

    /// Maximum number of retry attempts (default: 3)
    pub max_retries: u32,

    /// Enable gzip compression (default: true)
    pub enable_compression: bool,

    /// Retry delay strategy
    pub retry_delay: RetryDelay,

    /// Enable detailed logging (default: false).
    ///
    /// **Note:** This field is retained for API stability but is now a no-op.
    /// The transport pipeline always emits a masked `trace!` line for the
    /// outbound/inbound body (observable with `RUST_LOG=trace`), and `warn!`
    /// on retries — per the library-silent logging policy, the success path
    /// produces no `info!` output.
    pub enable_logging: bool,

    /// Enable sensitive data masking in logs (default: true)
    pub mask_sensitive_data: bool,
}

impl Default for HttpClientConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(60),
            max_retries: 3,
            enable_compression: true,
            retry_delay: RetryDelay::default(),
            enable_logging: false,
            mask_sensitive_data: true,
        }
    }
}

impl HttpClientConfig {
    /// Create a new builder for fluent configuration
    pub fn builder() -> HttpClientConfigBuilder {
        HttpClientConfigBuilder::new()
    }
}

/// Builder for creating `HttpClientConfig` instances.
///
/// Provides a fluent API for configuring HTTP client behavior.
///
/// # Example
///
/// ```ignore
/// use zai_rs::client::http::HttpClientConfig;
///
/// let config = HttpClientConfig::builder()
///     .max_retries(5)
///     .timeout(Duration::from_secs(120))
///     .retry_delay(RetryDelay::exponential(Duration::from_millis(100), Duration::from_secs(10)))
///     .build();
/// ```
pub struct HttpClientConfigBuilder {
    config: HttpClientConfig,
}

impl HttpClientConfigBuilder {
    /// Create a new builder with default configuration
    pub fn new() -> Self {
        Self {
            config: HttpClientConfig::default(),
        }
    }

    /// Set the request timeout duration
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.config.timeout = timeout;
        self
    }

    /// Set the maximum number of retry attempts
    pub fn max_retries(mut self, max_retries: u32) -> Self {
        self.config.max_retries = max_retries;
        self
    }

    /// Enable or disable gzip compression
    pub fn compression(mut self, enable: bool) -> Self {
        self.config.enable_compression = enable;
        self
    }

    /// Set the retry delay strategy
    pub fn retry_delay(mut self, delay: RetryDelay) -> Self {
        self.config.retry_delay = delay;
        self
    }

    /// Enable or disable detailed logging (retained for API stability; no-op —
    /// see [`HttpClientConfig::enable_logging`]).
    pub fn logging(mut self, enable: bool) -> Self {
        self.config.enable_logging = enable;
        self
    }

    /// Enable or disable sensitive data masking in logs
    pub fn mask_sensitive_data(mut self, enable: bool) -> Self {
        self.config.mask_sensitive_data = enable;
        self
    }

    /// Build the configuration
    pub fn build(self) -> HttpClientConfig {
        self.config
    }
}

impl Default for HttpClientConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// A global HTTP client registry for connection pooling and configuration
/// caching. The cached value is the build *result* so a persistent init
/// failure is remembered and surfaced rather than re-attempted per request.
static HTTP_CLIENTS: OnceLock<dashmap::DashMap<String, Arc<Result<reqwest::Client, ZaiError>>>> =
    OnceLock::new();

/// Get or create an HTTP client with the specified configuration.
///
/// Clients are cached by configuration to allow connection reuse. The build
/// *result* itself is cached, so a persistent client-init failure (e.g. a TLS
/// provider initialization error) is surfaced as a [`ZaiError`] instead of
/// panicking the whole process on the first request.
pub fn http_client_with_config(config: &HttpClientConfig) -> ZaiResult<reqwest::Client> {
    let config_key = format!(
        "timeout:{:?}|compression:{}",
        config.timeout, config.enable_compression
    );

    let clients = HTTP_CLIENTS.get_or_init(dashmap::DashMap::new);

    let cached = clients
        .entry(config_key)
        .or_insert_with(|| Arc::new(build_reqwest_client(config)));

    match &**cached {
        Ok(client) => Ok(client.clone()),
        Err(err) => Err(err.clone()),
    }
}

/// Construct a `reqwest::Client` from an [`HttpClientConfig`].
///
/// When compression is enabled (the default), reqwest advertises an
/// `Accept-Encoding: gzip` header and transparently decompresses gzip
/// responses; `enable_compression` is wired through to the builder.
fn build_reqwest_client(config: &HttpClientConfig) -> ZaiResult<reqwest::Client> {
    let mut builder = reqwest::Client::builder().timeout(config.timeout);
    if config.enable_compression {
        builder = builder.gzip(true);
    }
    builder.build().map_err(ZaiError::from)
}

/// Parse a successful HTTP response into a typed value.
///
/// The raw response body is captured as text before deserialization so it can
/// be emitted at `trace` level. This is the single place that surfaces the
/// received wire payload — the streaming/SSE paths log per-chunk, and the
/// error path logs the body inline with the retry decision.
///
/// The body is run through [`mask_sensitive_info`] before logging so secrets
/// (API keys, bearer tokens, …) never leak into logs even at `trace` level.
#[tracing::instrument(
    name = "http.response",
    skip_all,
    fields(
        otel.name = "http.parse_response",
        http.url = tracing::field::Empty,
        http.status_code = tracing::field::Empty,
    )
)]
pub async fn parse_typed_response<T>(resp: reqwest::Response) -> ZaiResult<T>
where
    T: DeserializeOwned,
{
    let status = resp.status();
    let url = resp.url().to_string();

    let span = tracing::Span::current();
    span.record("http.url", url.as_str());
    span.record("http.status_code", status.as_u16());

    let body = resp.text().await.map_err(ZaiError::from)?;

    trace!(
        url = %url,
        http_status = %status,
        bytes = body.len(),
        response_body = %mask_sensitive_info(&body),
        "Received HTTP response body"
    );

    serde_json::from_str::<T>(&body).map_err(ZaiError::from)
}

/// Send a JSON request through the shared transport pipeline.
///
/// Emits a single always-on **`trace`** line carrying the raw sent JSON body
/// (masked via [`mask_sensitive_info`]), so the wire payload is observable
/// with `RUST_LOG=trace`. Per the library-silent logging policy, the success
/// path produces no higher-level output; only retries are surfaced (`warn!`).
///
/// `enable_logging` on [`HttpClientConfig`] is retained for API stability but
/// no longer adds output — the `trace` line already covers that need.
/// Send a JSON request through the shared transport pipeline.
///
/// The body is serialized exactly once; the retry loop then clones cheap
/// `Bytes`/`Arc<str>` handles per attempt rather than re-serializing or
/// deep-copying the body string.
///
/// Emits a single always-on **`trace`** line carrying the raw sent JSON body
/// (masked via [`mask_sensitive_info`]), so the wire payload is observable
/// with `RUST_LOG=trace`. Per the library-silent logging policy, the success
/// path produces no higher-level output; only retries are surfaced (`warn!`).
pub async fn send_json_request<T>(
    method: Method,
    url: impl Into<String>,
    api_key: impl AsRef<str>,
    body: &T,
    config: Arc<HttpClientConfig>,
) -> ZaiResult<reqwest::Response>
where
    T: serde::Serialize + ?Sized,
{
    let serialized = serde_json::to_string(body).map_err(|e| ZaiError::JsonError(Arc::new(e)))?;
    send_request_bytes(
        method,
        url.into(),
        Arc::from(api_key.as_ref()),
        Bytes::from(serialized),
        config,
    )
    .await
}

/// Run a pre-serialized JSON body through the retry pipeline.
///
/// This is the single retry entry for JSON-with-body requests: the public
/// [`send_json_request`] and the `HttpClient::post`/`put` defaults both
/// serialize once and hand the resulting bytes here, so neither
/// double-serialization nor per-retry body clones occur.
#[tracing::instrument(
    name = "http.request",
    skip_all,
    fields(
        otel.name = "http.send_json",
        http.method = %method,
        http.url = tracing::field::Empty,
    )
)]
async fn send_request_bytes(
    method: Method,
    url: String,
    api_key: Arc<str>,
    body: Bytes,
    config: Arc<HttpClientConfig>,
) -> ZaiResult<reqwest::Response> {
    tracing::Span::current().record("http.url", url.as_str());

    // Always-on `trace` line for the raw outbound body (masked). This is what
    // `RUST_LOG=trace` surfaces so developers can see exactly what is sent.
    let body_str = std::str::from_utf8(&body).unwrap_or("");
    trace!(
        method = %method,
        url = %url,
        bytes = body.len(),
        request_body = %mask_sensitive_info(body_str),
        "Sending HTTP request body"
    );

    send_with_retry_factory(&config, move |client| {
        Ok(client
            .request(method.clone(), &url)
            .bearer_auth(api_key.as_ref())
            .header("Content-Type", "application/json")
            .body(body.clone()))
    })
    .await
}

/// Send a request without a JSON body through the shared transport pipeline.
///
/// Always emits a `trace` line for the outbound request line (no body to log),
/// so GET/DELETE traffic is observable with `RUST_LOG=trace`.
#[tracing::instrument(
    name = "http.request",
    skip_all,
    fields(
        otel.name = "http.send_empty",
        http.method = %method,
        http.url = tracing::field::Empty,
    )
)]
pub async fn send_empty_request(
    method: Method,
    url: impl Into<String>,
    api_key: impl AsRef<str>,
    config: Arc<HttpClientConfig>,
) -> ZaiResult<reqwest::Response> {
    let url_value: String = url.into();
    tracing::Span::current().record("http.url", url_value.as_str());
    trace!(
        method = %method,
        url = %url_value,
        request_body = %"",
        "Sending HTTP request (no body)"
    );
    let key: Arc<str> = Arc::from(api_key.as_ref());
    send_with_retry_factory(&config, move |client| {
        Ok(client
            .request(method.clone(), &url_value)
            .bearer_auth(key.as_ref()))
    })
    .await
}

/// Send a multipart/form-data request through the shared transport pipeline.
///
/// The form is built per attempt so retry can safely recreate multipart bodies.
///
/// Always emits a `trace` line for the outbound request metadata (multipart
/// bodies are not serialized as JSON, so only the request line is logged).
#[tracing::instrument(
    name = "http.request",
    skip_all,
    fields(
        otel.name = "http.send_multipart",
        http.method = %method,
        http.url = tracing::field::Empty,
    )
)]
pub async fn send_multipart_request<F>(
    method: Method,
    url: impl Into<String>,
    api_key: impl AsRef<str>,
    config: Arc<HttpClientConfig>,
    mut build_form: F,
) -> ZaiResult<reqwest::Response>
where
    F: FnMut() -> ZaiResult<reqwest::multipart::Form> + Send,
{
    let url_value: String = url.into();
    tracing::Span::current().record("http.url", url_value.as_str());
    trace!(
        method = %method,
        url = %url_value,
        "Sending multipart HTTP request"
    );
    let key: Arc<str> = Arc::from(api_key.as_ref());
    send_with_retry_factory(&config, move |client| {
        let form = build_form()?;
        Ok(client
            .request(method.clone(), &url_value)
            .bearer_auth(key.as_ref())
            .multipart(form))
    })
    .await
}

/// Trait for HTTP clients that communicate with the Zhipu AI API.
///
/// Every concrete request builder in the SDK implements this so the shared
/// transport helpers can post/submit requests uniformly.
pub trait HttpClient {
    /// Request body type (must be JSON-serializable).
    type Body: serde::Serialize;
    /// Resolved API URL holder (typically `String` or `&'a str`).
    type ApiUrl: AsRef<str>;
    /// API key holder (typically `String` or `&'a str`).
    type ApiKey: AsRef<str>;

    /// Resolved target URL for the request.
    fn api_url(&self) -> &Self::ApiUrl;
    /// API key used for `Authorization: Bearer …`.
    fn api_key(&self) -> &Self::ApiKey;
    /// Serialized request body.
    fn body(&self) -> &Self::Body;

    /// Get HTTP client configuration for this request
    ///
    /// Override this method to provide custom configuration.
    /// Default implementation returns default configuration.
    fn http_config(&self) -> Arc<HttpClientConfig> {
        static DEFAULT: std::sync::OnceLock<Arc<HttpClientConfig>> = std::sync::OnceLock::new();
        DEFAULT
            .get_or_init(|| Arc::new(HttpClientConfig::default()))
            .clone()
    }

    /// Sends a POST request to the API endpoint.
    ///
    /// This method implements retry logic with exponential backoff and jitter.
    /// It supports configuration through `http_config` method.
    fn post(&self) -> impl std::future::Future<Output = ZaiResult<reqwest::Response>> + Send {
        let config = self.http_config().clone();
        let url = self.api_url().as_ref().to_owned();
        let key: Arc<str> = Arc::from(self.api_key().as_ref());
        // Serialize once (to bytes); `send_request_bytes` reuses the cheap
        // `Bytes` handle across retries instead of re-serializing per attempt.
        let body = serde_json::to_vec(self.body()).map_err(|e| ZaiError::JsonError(Arc::new(e)));

        async move {
            let body = body?;
            send_request_bytes(Method::POST, url, key, Bytes::from(body), config).await
        }
    }

    /// Sends a GET request to the API endpoint.
    ///
    /// This method implements retry logic with exponential backoff and jitter.
    /// It supports configuration through the `http_config` method.
    fn get(&self) -> impl std::future::Future<Output = ZaiResult<reqwest::Response>> + Send {
        let config = self.http_config().clone();
        let url = self.api_url().as_ref().to_owned();
        let key: Arc<str> = Arc::from(self.api_key().as_ref());

        async move { send_empty_request(Method::GET, url, key, config).await }
    }

    /// Sends a PUT request with a JSON body to the API endpoint.
    fn put(&self) -> impl std::future::Future<Output = ZaiResult<reqwest::Response>> + Send {
        let config = self.http_config().clone();
        let url = self.api_url().as_ref().to_owned();
        let key: Arc<str> = Arc::from(self.api_key().as_ref());
        let body = serde_json::to_vec(self.body()).map_err(|e| ZaiError::JsonError(Arc::new(e)));

        async move {
            let body = body?;
            send_request_bytes(Method::PUT, url, key, Bytes::from(body), config).await
        }
    }

    /// Sends a DELETE request without a body to the API endpoint.
    fn delete(&self) -> impl std::future::Future<Output = ZaiResult<reqwest::Response>> + Send {
        let config = self.http_config().clone();
        let url = self.api_url().as_ref().to_owned();
        let key: Arc<str> = Arc::from(self.api_key().as_ref());

        async move { send_empty_request(Method::DELETE, url, key, config).await }
    }
}

/// Internal helper: executes retryable request builders.
#[tracing::instrument(
    skip(config, build_request),
    fields(max_retries = config.max_retries, attempt = tracing::field::Empty)
)]
async fn send_with_retry_factory<F>(
    config: &HttpClientConfig,
    mut build_request: F,
) -> ZaiResult<reqwest::Response>
where
    F: FnMut(reqwest::Client) -> ZaiResult<reqwest::RequestBuilder>,
{
    let mut last_error: Option<ZaiError> = None;
    let client = http_client_with_config(config)?;

    for attempt in 0..=config.max_retries {
        tracing::Span::current().record("attempt", attempt);
        let resp = match build_request(client.clone()) {
            Ok(builder) => builder.send().await,
            Err(error) => return Err(error),
        };

        match resp {
            Ok(resp) => {
                let status = resp.status();
                let url = resp.url().to_string();

                if status.is_success() {
                    trace!(http_status = %status, url = %url, "Request succeeded");
                    return Ok(resp);
                }

                let text = resp.text().await.unwrap_or_default();
                let error = parse_api_error_response(status.as_u16(), text);

                if should_retry(&error, attempt, config.max_retries) {
                    last_error = Some(error.clone());
                    let delay = calculate_retry_delay(attempt, &config.retry_delay);
                    let delay_with_jitter = add_jitter(delay);
                    warn!(
                        url = %url,
                        http_status = %status,
                        attempt = attempt + 1,
                        max_attempts = config.max_retries + 1,
                        retry_delay = ?delay_with_jitter,
                        error = %error.compact(),
                        "Request failed, retrying"
                    );
                    tokio::time::sleep(delay_with_jitter).await;
                } else {
                    return Err(error);
                }
            },
            Err(e) => {
                let url = e.url().map(|u| u.to_string()).unwrap_or_default();
                let error = ZaiError::from(e);

                if should_retry(&error, attempt, config.max_retries) {
                    last_error = Some(error.clone());
                    let delay = calculate_retry_delay(attempt, &config.retry_delay);
                    let delay_with_jitter = add_jitter(delay);
                    warn!(
                        url = %url,
                        attempt = attempt + 1,
                        max_attempts = config.max_retries + 1,
                        retry_delay = ?delay_with_jitter,
                        error = %error.compact(),
                        "Request failed, retrying"
                    );
                    tokio::time::sleep(delay_with_jitter).await;
                } else {
                    return Err(error);
                }
            },
        }
    }

    let final_err = last_error.unwrap_or_else(|| ZaiError::HttpError {
        status: 500,
        message: "Unknown error after retries".to_string(),
    });
    warn!(
        code = ?final_err.code(),
        error = %final_err.compact(),
        "Request failed after all retries"
    );
    Err(final_err)
}

/// Calculate delay for a retry attempt based on retry delay strategy.
fn calculate_retry_delay(attempt: u32, strategy: &RetryDelay) -> Duration {
    match strategy {
        RetryDelay::Fixed(delay) => *delay,
        RetryDelay::Exponential { base, max } => {
            let delay = *base * 2u32.pow(attempt.min(10));
            delay.min(*max)
        },
        RetryDelay::None => Duration::ZERO,
    }
}

/// Determines if an error should trigger a retry.
///
/// Thin wrapper over [`ZaiError::is_retryable`] that also enforces the
/// attempt-count budget.
fn should_retry(error: &ZaiError, attempt: u32, max_retries: u32) -> bool {
    attempt < max_retries && error.is_retryable()
}

/// Adds jitter to delay to avoid thundering herd.
fn add_jitter(delay: Duration) -> Duration {
    let jitter_ms = fastrand::u64(0..=delay.as_millis() as u64 / 4);
    delay + Duration::from_millis(jitter_ms)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_code_display_num() {
        let code = ErrorCode::Num(123);
        assert_eq!(format!("{}", code), "123");
    }

    #[test]
    fn test_error_code_display_str() {
        let code = ErrorCode::Str("auth_error".to_string());
        assert_eq!(format!("{}", code), "auth_error");
    }

    #[test]
    fn test_to_api_code_num() {
        let code = ErrorCode::Num(401);
        assert_eq!(to_api_code(&code), 401);
    }

    #[test]
    fn test_to_api_code_str_valid() {
        let code = ErrorCode::Str("429".to_string());
        assert_eq!(to_api_code(&code), 429);
    }

    #[test]
    fn test_to_api_code_str_invalid() {
        let code = ErrorCode::Str("invalid".to_string());
        assert_eq!(to_api_code(&code), 0);
    }

    #[test]
    fn test_to_api_code_num_overflow() {
        let code = ErrorCode::Num(99999);
        assert_eq!(to_api_code(&code), 0);
    }

    #[test]
    fn test_api_error_envelope_deserialize() {
        let json = r#"{"error":{"code":401,"message":"Unauthorized"}}"#;
        let envelope: ApiErrorEnvelope = serde_json::from_str(json).unwrap();
        let (_code, message) = envelope.into_parts();
        assert_eq!(message, "Unauthorized");
    }

    #[test]
    fn test_api_error_envelope_deserialize_str_code() {
        let json = r#"{"error":{"code":"1302","message":"Rate limit exceeded"}}"#;
        let envelope: ApiErrorEnvelope = serde_json::from_str(json).unwrap();
        let (code, message) = envelope.into_parts();
        assert_eq!(message, "Rate limit exceeded");
        assert_eq!(to_api_code(&code), 1302);
    }

    #[test]
    fn test_api_error_envelope_deserialize_flat() {
        let json = r#"{"code":1312,"message":"Quota exhausted"}"#;
        let envelope: ApiErrorEnvelope = serde_json::from_str(json).unwrap();
        let (code, message) = envelope.into_parts();
        assert_eq!(message, "Quota exhausted");
        assert_eq!(to_api_code(&code), 1312);
    }

    #[test]
    fn test_parse_api_error_response_prefers_business_code() {
        let error =
            parse_api_error_response(429, r#"{"code":1312,"message":"Quota exhausted"}"#.into());
        assert!(matches!(
            error,
            ZaiError::RateLimitError {
                code: 1312,
                message
            } if message == "Quota exhausted"
        ));
    }

    #[test]
    fn test_parse_api_error_response_unparseable_body() {
        let error = parse_api_error_response(500, "not json".to_string());
        assert!(matches!(
            error,
            ZaiError::HttpError {
                status: 500,
                message
            } if message == "Internal server error - try again later"
        ));
    }

    #[test]
    fn test_calculate_retry_delay_fixed() {
        let delay = Duration::from_secs(2);
        let strategy = RetryDelay::Fixed(delay);
        assert_eq!(calculate_retry_delay(0, &strategy), delay);
        assert_eq!(calculate_retry_delay(1, &strategy), delay);
        assert_eq!(calculate_retry_delay(5, &strategy), delay);
    }

    #[test]
    fn test_calculate_retry_delay_exponential() {
        let base = Duration::from_millis(500);
        let max = Duration::from_secs(5);
        let strategy = RetryDelay::Exponential { base, max };

        assert_eq!(
            calculate_retry_delay(0, &strategy),
            Duration::from_millis(500)
        );
        assert_eq!(
            calculate_retry_delay(1, &strategy),
            Duration::from_millis(1000)
        );
        assert_eq!(
            calculate_retry_delay(2, &strategy),
            Duration::from_millis(2000)
        );
        assert_eq!(
            calculate_retry_delay(3, &strategy),
            Duration::from_millis(4000)
        );
        assert_eq!(calculate_retry_delay(4, &strategy), max);
        assert_eq!(calculate_retry_delay(10, &strategy), max);
    }

    #[test]
    fn test_calculate_retry_delay_none() {
        let strategy = RetryDelay::None;
        assert_eq!(calculate_retry_delay(0, &strategy), Duration::ZERO);
        assert_eq!(calculate_retry_delay(5, &strategy), Duration::ZERO);
    }

    #[test]
    fn test_add_jitter() {
        let delay = Duration::from_millis(1000);
        let with_jitter = add_jitter(delay);

        // Jitter should be between 0 and 25% of the delay
        assert!(with_jitter >= delay);
        assert!(with_jitter <= delay + Duration::from_millis(250));
    }

    #[test]
    fn test_should_retry_server_error() {
        let error = ZaiError::HttpError {
            status: 500,
            message: "Internal server error".to_string(),
        };
        assert!(should_retry(&error, 0, 3));
        assert!(should_retry(&error, 2, 3));
        assert!(!should_retry(&error, 3, 3));
    }

    #[test]
    fn test_should_retry_gateway_timeout() {
        let error = ZaiError::HttpError {
            status: 504,
            message: "Gateway timeout".to_string(),
        };
        assert!(should_retry(&error, 0, 3));
    }

    #[test]
    fn test_should_retry_rate_limit() {
        let error = ZaiError::RateLimitError {
            code: 1302,
            message: "Rate limit exceeded".to_string(),
        };
        assert!(should_retry(&error, 0, 3));
    }

    #[test]
    fn test_should_not_retry_content_policy_error() {
        let error = ZaiError::ContentPolicyError {
            code: 1301,
            message: "Unsafe content detected".to_string(),
        };
        assert!(!should_retry(&error, 0, 3));
    }

    #[test]
    fn test_should_retry_http_429() {
        let error = ZaiError::HttpError {
            status: 429,
            message: "Too many requests".to_string(),
        };
        assert!(should_retry(&error, 0, 3));
    }

    #[test]
    fn test_should_retry_network_error() {
        // Since we can't construct reqwest::Error directly in tests,
        // simulate network error behavior with a 503 status
        let error = ZaiError::HttpError {
            status: 503,
            message: "Network error".to_string(),
        };
        assert!(should_retry(&error, 0, 3));
    }

    #[test]
    fn test_should_not_retry_client_error() {
        let error = ZaiError::HttpError {
            status: 400,
            message: "Bad request".to_string(),
        };
        assert!(!should_retry(&error, 0, 3));
    }

    #[test]
    fn test_should_not_retry_unauthorized() {
        let error = ZaiError::AuthError {
            code: 1001,
            message: "Invalid API key".to_string(),
        };
        assert!(!should_retry(&error, 0, 3));
    }

    #[test]
    fn test_should_not_retry_account_error() {
        let error = ZaiError::AccountError {
            code: 1110,
            message: "Account not found".to_string(),
        };
        assert!(!should_retry(&error, 0, 3));
    }

    #[test]
    fn test_should_not_retry_not_found() {
        let error = ZaiError::HttpError {
            status: 404,
            message: "Resource not found".to_string(),
        };
        assert!(!should_retry(&error, 0, 3));
    }

    #[test]
    fn test_should_not_retry_sdk_timeout() {
        // Regression guard: a client-side polling timeout (SDK_TIMEOUT) must NOT
        // be auto-retried. Previously this surfaced as RateLimitError{code:0},
        // which should_retry treats as retryable.
        let error = ZaiError::ApiError {
            code: crate::client::error::codes::SDK_TIMEOUT,
            message: "Timeout waiting for parsing result".to_string(),
        };
        assert!(!should_retry(&error, 0, 3));
    }

    #[test]
    fn test_http_client_config_default() {
        let config = HttpClientConfig::default();
        assert_eq!(config.timeout, Duration::from_secs(60));
        assert_eq!(config.max_retries, 3);
        assert!(config.enable_compression);
        matches!(config.retry_delay, RetryDelay::Exponential { .. });
    }

    #[test]
    fn test_compression_config_round_trips_through_builder() {
        // Default: compression on.
        let on = HttpClientConfig::default();
        assert!(on.enable_compression);

        // Builder can disable it.
        let off = HttpClientConfig::builder().compression(false).build();
        assert!(!off.enable_compression);

        // The cached-client key encodes the compression flag, so the two
        // configs resolve to distinct pooled clients (otherwise toggling
        // compression would be a silent no-op due to caching).
        let client_on = http_client_with_config(&on).expect("client build with compression");
        let client_off = http_client_with_config(&off).expect("client build without compression");
        // reqwest::Client is neither Eq nor introspectable, but the registry
        // guarantees a distinct Client per distinct key — exercise the path.
        let _ = (client_on, client_off);
    }

    #[test]
    fn test_retry_delay_default() {
        let delay = RetryDelay::default();
        matches!(delay, RetryDelay::Exponential { base, max } if base == Duration::from_millis(500) && max == Duration::from_secs(5));
    }
}
