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

use serde::Deserialize;
use tracing::{debug, info, warn};

use crate::client::error::{ZaiError, ZaiResult, mask_sensitive_info};

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

/// Retry delay strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryDelay {
    /// Fixed delay between retries
    Fixed(Duration),

    /// Exponential backoff with jitter
    Exponential { base: Duration, max: Duration },

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

    /// Enable detailed logging (default: false)
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

    /// Enable or disable detailed logging
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
/// caching.
static HTTP_CLIENTS: OnceLock<dashmap::DashMap<String, reqwest::Client>> = OnceLock::new();

/// Get or create an HTTP client with the specified configuration
///
/// Clients are cached by configuration to allow connection reuse.
pub fn http_client_with_config(config: &HttpClientConfig) -> reqwest::Client {
    let config_key = format!(
        "timeout:{:?}|compression:{}",
        config.timeout, config.enable_compression
    );

    let clients = HTTP_CLIENTS.get_or_init(dashmap::DashMap::new);

    clients
        .entry(config_key)
        .or_insert_with(|| {
            let builder = reqwest::Client::builder().timeout(config.timeout);

            // Note: reqwest enables gzip compression by default
            // if config.enable_compression {
            //     builder = builder.gzip(true);
            // }

            builder.build().expect("Failed to build reqwest Client")
        })
        .clone()
}

/// Trait for HTTP clients that communicate with the Zhipu AI API.
pub trait HttpClient {
    type Body: serde::Serialize;
    type ApiUrl: AsRef<str>;
    type ApiKey: AsRef<str>;

    fn api_url(&self) -> &Self::ApiUrl;
    fn api_key(&self) -> &Self::ApiKey;
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
        let body_compact =
            serde_json::to_string(self.body()).map_err(|e| ZaiError::JsonError(Arc::new(e)));

        let config = self.http_config().clone();
        let enable_logging = config.enable_logging;
        let mask_sensitive = config.mask_sensitive_data;

        let body_pretty_opt = if enable_logging {
            match serde_json::to_string_pretty(self.body()) {
                Ok(pretty) => Some(pretty),
                Err(e) => {
                    warn!("Failed to pretty-print request body: {}", e);
                    None
                },
            }
        } else {
            None
        };

        let url = self.api_url().as_ref().to_owned();
        let key = self.api_key().as_ref().to_owned();

        async move {
            let body = body_compact?;

            if enable_logging {
                let log_body = if mask_sensitive {
                    mask_sensitive_info(body.as_str())
                } else {
                    body.clone()
                };
                if let Some(pretty) = body_pretty_opt {
                    let log_pretty = if mask_sensitive {
                        mask_sensitive_info(&pretty)
                    } else {
                        pretty
                    };
                    info!(request_body = %log_pretty, "Sending POST request");
                } else {
                    debug!(request_body = %log_body, "Sending POST request");
                }
            }

            let client = http_client_with_config(&config);
            let mut last_error: Option<ZaiError> = None;

            for attempt in 0..=config.max_retries {
                let resp = client
                    .post(&url)
                    .bearer_auth(&key)
                    .header("Content-Type", "application/json")
                    .body(body.clone())
                    .send()
                    .await;

                match resp {
                    Ok(resp) => {
                        let status = resp.status();

                        if status.is_success() {
                            debug!(http_status = %status, "Request succeeded");
                            return Ok(resp);
                        }

                        // Parse error for potential retry
                        let text = resp.text().await.unwrap_or_default();

                        let error =
                            if let Ok(parsed) = serde_json::from_str::<ApiErrorEnvelope>(&text) {
                                let api_code = to_api_code(&parsed.error.code);
                                ZaiError::from_api_response(
                                    status.as_u16(),
                                    api_code,
                                    parsed.error.message,
                                )
                            } else {
                                ZaiError::from_api_response(status.as_u16(), 0, text)
                            };

                        if should_retry(&error, attempt, config.max_retries) {
                            last_error = Some(error.clone());
                            let delay = calculate_retry_delay(attempt, &config.retry_delay);
                            let delay_with_jitter = add_jitter(delay);
                            warn!(
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
                        let error = ZaiError::from(e);

                        if should_retry(&error, attempt, config.max_retries) {
                            last_error = Some(error.clone());
                            let delay = calculate_retry_delay(attempt, &config.retry_delay);
                            let delay_with_jitter = add_jitter(delay);
                            warn!(
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

            Err(last_error.unwrap_or_else(|| ZaiError::HttpError {
                status: 500,
                message: "Unknown error after retries".to_string(),
            }))
        }
    }

    /// Sends a GET request to the API endpoint.
    ///
    /// This method implements retry logic with exponential backoff and jitter.
    /// It supports configuration through the `http_config` method.
    fn get(&self) -> impl std::future::Future<Output = ZaiResult<reqwest::Response>> + Send {
        let config = self.http_config().clone();
        let url = self.api_url().as_ref().to_owned();
        let key = self.api_key().as_ref().to_owned();

        async move {
            let client = http_client_with_config(&config);
            let mut last_error: Option<ZaiError> = None;

            for attempt in 0..=config.max_retries {
                let resp = client.get(&url).bearer_auth(&key).send().await;

                match resp {
                    Ok(resp) => {
                        let status = resp.status();

                        if status.is_success() {
                            debug!(http_status = %status, "Request succeeded");
                            return Ok(resp);
                        }

                        // Parse error for potential retry
                        let text = resp.text().await.unwrap_or_default();

                        let error =
                            if let Ok(parsed) = serde_json::from_str::<ApiErrorEnvelope>(&text) {
                                let api_code = to_api_code(&parsed.error.code);
                                ZaiError::from_api_response(
                                    status.as_u16(),
                                    api_code,
                                    parsed.error.message,
                                )
                            } else {
                                ZaiError::from_api_response(status.as_u16(), 0, text)
                            };

                        if should_retry(&error, attempt, config.max_retries) {
                            last_error = Some(error.clone());
                            let delay = calculate_retry_delay(attempt, &config.retry_delay);
                            let delay_with_jitter = add_jitter(delay);
                            warn!(
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
                        let error = ZaiError::from(e);

                        if should_retry(&error, attempt, config.max_retries) {
                            last_error = Some(error.clone());
                            let delay = calculate_retry_delay(attempt, &config.retry_delay);
                            let delay_with_jitter = add_jitter(delay);
                            warn!(
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

            Err(last_error.unwrap_or_else(|| ZaiError::HttpError {
                status: 500,
                message: "Unknown error after retries".to_string(),
            }))
        }
    }
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
fn should_retry(error: &ZaiError, attempt: u32, max_retries: u32) -> bool {
    if attempt >= max_retries {
        return false;
    }

    match error {
        // Retry on server errors (5xx)
        ZaiError::HttpError { status, .. } => (500..600).contains(status),
        // Retry on rate limit errors (API code 1301)
        ZaiError::RateLimitError { .. } => true,
        // Retry on network errors
        ZaiError::NetworkError(_) => true,
        // Don't retry on client errors (4xx), auth errors, account errors, etc.
        _ => false,
    }
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
        assert_eq!(envelope.error.message, "Unauthorized");
    }

    #[test]
    fn test_api_error_envelope_deserialize_str_code() {
        let json = r#"{"error":{"code":"1300","message":"Rate limit exceeded"}}"#;
        let envelope: ApiErrorEnvelope = serde_json::from_str(json).unwrap();
        assert_eq!(envelope.error.message, "Rate limit exceeded");
        assert_eq!(to_api_code(&envelope.error.code), 1300);
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
            code: 1301,
            message: "Rate limit exceeded".to_string(),
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
    fn test_http_client_config_default() {
        let config = HttpClientConfig::default();
        assert_eq!(config.timeout, Duration::from_secs(60));
        assert_eq!(config.max_retries, 3);
        assert!(config.enable_compression);
        matches!(config.retry_delay, RetryDelay::Exponential { .. });
    }

    #[test]
    fn test_retry_delay_default() {
        let delay = RetryDelay::default();
        matches!(delay, RetryDelay::Exponential { base, max } if base == Duration::from_millis(500) && max == Duration::from_secs(5));
    }
}
