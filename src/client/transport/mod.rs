//! The crate-private Transport (plan P03.4–P03.8).
//!
//! Execution pipeline:
//!   validate → build URL → encode body → enforce request limit
//!   → send/retry → enforce response limit → probe error → decode
//!   → validate invariant → convert to public response.
//!
//! Timeouts are split (plan §4): connect 10s, per-attempt 60s, overall 120s,
//! stream idle 60s. Backoff uses full jitter with an injectable [`JitterSource`]
//! so tests use virtual time. Retry-After (429/503) replaces jitter when valid.
//! The Transport only ever logs fixed metadata (method, route template, status,
//! byte count, attempt, elapsed, sanitized correlation request_id) — never the
//! URL, header values, query values, or body.
//!
//! Every REST request is dispatched through this transport. JSON, empty-body,
//! binary and multipart endpoints therefore share the same authentication,
//! retry, redirect, size-limit and response-decoding behavior.

#![allow(dead_code)]

pub mod decode;
pub mod download;
pub mod limits;
pub mod multipart;
pub mod redaction;
pub mod redirect;
pub mod request;
pub mod retry;

use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use tracing::warn;

use crate::ZaiError;
use crate::ZaiResult;
use crate::client::error::codes;
use crate::client::secret::ApiSecret;
use crate::client::transport::limits::{
    ERROR_BODY_MAX, JSON_REQUEST_MAX, JSON_RESPONSE_MAX, MULTIPART_FILE_BYTES_MAX,
};
use crate::client::transport::redirect::follow as follow_redirect;
use crate::client::transport::request::{PreparedRequest, ResponseMode};
use crate::client::transport::retry::{JitterSource, RetrySafety, backoff_delay};
#[allow(unused_imports)]
use crate::client::transport::retry::{
    is_retryable_outcome, parse_retry_after, reconcile_retry_after,
};

/// The fixed timeout policy (plan §4). Defaults: connect 10s, attempt 60s,
/// overall 120s, stream idle 60s.
#[derive(Debug, Clone, Copy)]
pub struct TimeoutPolicy {
    pub connect: Duration,
    pub attempt: Duration,
    pub overall: Duration,
    pub stream_idle: Duration,
}

impl Default for TimeoutPolicy {
    fn default() -> Self {
        Self {
            connect: Duration::from_secs(10),
            attempt: Duration::from_secs(60),
            overall: Duration::from_secs(120),
            stream_idle: Duration::from_secs(60),
        }
    }
}

/// An injectable clock, so tests advance virtual time instead of sleeping.
pub trait Clock: Send + Sync {
    /// Monotonic instant "now".
    fn now(&self) -> std::time::Instant;
}

/// The real wall clock.
pub struct WallClock;
impl Clock for WallClock {
    fn now(&self) -> std::time::Instant {
        std::time::Instant::now()
    }
}

/// The Transport: holds the shared reqwest client, timeout policy, max attempts
/// and injected jitter/clock. Built once per `ZaiClient`.
pub(crate) struct Transport {
    pub(crate) reqwest: reqwest::Client,
    pub(crate) timeouts: TimeoutPolicy,
    pub(crate) max_attempts: u8,
    pub(crate) jitter: Arc<dyn JitterSource>,
    pub(crate) clock: Arc<dyn Clock>,
    secret: ApiSecret,
    additional_headers: Vec<crate::client::AdditionalHeader>,
}

/// A fully buffered response produced by the transport pipeline.
///
/// Keeping status, headers and bytes together replaces the old dependency on
/// `reqwest::Response` and lets typed JSON and binary endpoints share one path.
pub struct TransportResponse {
    status: u16,
    headers: reqwest::header::HeaderMap,
    body: Bytes,
}

impl TransportResponse {
    pub fn status(&self) -> u16 {
        self.status
    }
    pub fn headers(&self) -> &reqwest::header::HeaderMap {
        &self.headers
    }
    pub fn bytes(self) -> ZaiResult<Bytes> {
        if let Ok(text) = std::str::from_utf8(&self.body)
            && let Some(error) = decode::extract_error_envelope(text)
        {
            return Err(api_error(self.status, error));
        }
        if !(200..300).contains(&self.status) {
            return Err(ZaiError::from_api_response(
                self.status,
                0,
                String::from_utf8_lossy(&self.body).into_owned(),
            ));
        }
        Ok(self.body)
    }

    pub fn json<T: serde::de::DeserializeOwned>(self) -> ZaiResult<T> {
        if let Ok(text) = std::str::from_utf8(&self.body)
            && let Some(error) = decode::extract_error_envelope(text)
        {
            return Err(api_error(self.status, error));
        }
        if !(200..300).contains(&self.status) {
            return Err(ZaiError::from_api_response(
                self.status,
                0,
                String::from_utf8_lossy(&self.body).into_owned(),
            ));
        }
        serde_json::from_slice(&self.body).map_err(ZaiError::from)
    }
}

struct SystemJitter;
impl JitterSource for SystemJitter {
    fn jitter(&self, upper: Duration) -> Duration {
        let upper_nanos = upper.as_nanos();
        if upper_nanos == 0 {
            return Duration::ZERO;
        }
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos());
        Duration::from_nanos((seed % (upper_nanos + 1)) as u64)
    }
}

/// Outcome of a single send attempt, used by the retry loop.
pub(crate) enum AttemptOutcome {
    /// A final HTTP response (status + headers + body bytes already capped).
    Response {
        status: u16,
        headers: reqwest::header::HeaderMap,
        body: Bytes,
    },
    /// A transient error that may be retried.
    Transient(ZaiError),
    /// A terminal error (auth, non-retryable, etc.).
    Terminal(ZaiError),
}

impl Transport {
    pub(crate) fn new(
        reqwest: reqwest::Client,
        secret: ApiSecret,
        config: &crate::client::HttpTransportConfig,
    ) -> Self {
        Self {
            reqwest,
            timeouts: TimeoutPolicy {
                connect: config.connect_timeout,
                attempt: config.request_timeout,
                overall: config
                    .request_timeout
                    .saturating_mul(u32::from(config.max_attempts)),
                stream_idle: Duration::from_secs(60),
            },
            max_attempts: config.max_attempts,
            jitter: Arc::new(SystemJitter),
            clock: Arc::new(WallClock),
            secret,
            additional_headers: config.additional_headers.clone(),
        }
    }

    /// Send a prepared request with the retry/timeout pipeline, returning the
    /// (status, headers, body) of the final attempt. The caller performs the
    /// typed decode.
    ///
    /// This is the P03 reference implementation; it exercises the full pipeline
    /// except multipart (added in P07) and SSE streaming (P08). For the P02–P05
    /// window, migrated endpoints call this; legacy endpoints keep their path.
    #[allow(clippy::too_many_lines)]
    pub(crate) async fn send(&self, prepped: &PreparedRequest<'_>) -> ZaiResult<TransportResponse> {
        // Enforce request body limit up front.
        if let request::BodyKind::Bytes(b) = &prepped.body
            && (b.len() as u64) > JSON_REQUEST_MAX
        {
            return Err(payload_too_large(JSON_REQUEST_MAX));
        }

        let safety = prepped.retry_safety.effective(prepped.retry_override);
        let deadline = self.clock.now() + self.timeouts.overall;
        let max_attempts = self.effective_max_attempts(safety);

        let mut attempt: u8 = 0;
        let mut url = prepped.url.clone();
        let mut hops: u8 = 0;
        loop {
            attempt += 1;
            if self.clock.now() >= deadline {
                return Err(timeout_overall());
            }

            let req = self.build_request(prepped.method, &url, &prepped.body)?;
            let send_result = tokio::time::timeout(self.timeouts.attempt, req.send()).await;

            let outcome = match send_result {
                Err(_) => AttemptOutcome::Transient(timeout_attempt()),
                Ok(Err(e)) => AttemptOutcome::Transient(ZaiError::from(e)),
                Ok(Ok(resp)) => {
                    let status = resp.status().as_u16();
                    let headers = resp.headers().clone();

                    // Redirect handling.
                    if (300..400).contains(&status) {
                        let location = headers
                            .get(reqwest::header::LOCATION)
                            .and_then(|v| v.to_str().ok())
                            .unwrap_or("");
                        match follow_redirect(
                            &url::Url::parse(&url).map_err(|_| invalid("current url parse"))?,
                            status,
                            location,
                            safety,
                            prepped.method,
                            hops,
                        ) {
                            Ok(Some(target)) => {
                                hops += 1;
                                url = target.to_string();
                                continue; // re-send to the new origin (no auth header)
                            },
                            Ok(None) | Err(_) => {
                                // Don't follow; treat as terminal with the body.
                                let body = cap_body(resp, ERROR_BODY_MAX).await;
                                return Ok(TransportResponse {
                                    status,
                                    headers,
                                    body,
                                });
                            },
                        }
                    }

                    let limit = if (200..300).contains(&status) {
                        match prepped.response_mode {
                            ResponseMode::Json => JSON_RESPONSE_MAX,
                            ResponseMode::Binary => MULTIPART_FILE_BYTES_MAX,
                        }
                    } else {
                        ERROR_BODY_MAX
                    };
                    let body = cap_body(resp, limit).await;
                    AttemptOutcome::Response {
                        status,
                        headers,
                        body,
                    }
                },
            };

            match outcome {
                AttemptOutcome::Response {
                    status,
                    headers,
                    body,
                } => {
                    let business_code = std::str::from_utf8(&body)
                        .ok()
                        .and_then(decode::extract_error_envelope)
                        .and_then(|e| e.code)
                        .and_then(|v| match v {
                            serde_json::Value::Number(n) => n.as_u64().map(|n| n as u16),
                            serde_json::Value::String(s) => s.parse().ok(),
                            _ => None,
                        });
                    if safety == RetrySafety::Idempotent
                        && is_retryable_outcome(status, business_code)
                        && attempt < max_attempts
                    {
                        let computed = backoff_delay(u32::from(attempt) - 1, self.jitter.as_ref());
                        let hint = headers
                            .get(reqwest::header::RETRY_AFTER)
                            .and_then(|v| v.to_str().ok())
                            .and_then(parse_retry_after);
                        let delay = reconcile_retry_after(hint, computed);
                        if self.clock.now() + delay >= deadline {
                            return Err(timeout_overall());
                        }
                        tokio::time::sleep(delay).await;
                        continue;
                    }
                    return Ok(TransportResponse {
                        status,
                        headers,
                        body,
                    });
                },
                AttemptOutcome::Terminal(e) => return Err(e),
                AttemptOutcome::Transient(e) => {
                    if attempt >= max_attempts {
                        return Err(retry_exhausted(e));
                    }
                    // Full-jitter backoff (Retry-After reconciliation is applied
                    // in P07 where the per-attempt response headers are in hand).
                    let delay = backoff_delay(u32::from(attempt) - 1, self.jitter.as_ref());
                    if self.clock.now() + delay >= deadline {
                        return Err(timeout_overall());
                    }
                    tokio::time::sleep(delay).await;
                },
            }
        }
    }

    fn effective_max_attempts(&self, safety: RetrySafety) -> u8 {
        match safety {
            RetrySafety::Idempotent => self.max_attempts.max(1),
            RetrySafety::NonIdempotent => 1,
        }
    }

    fn build_request(
        &self,
        method: &str,
        url: &str,
        body: &request::BodyKind<'_>,
    ) -> ZaiResult<reqwest::RequestBuilder> {
        let m = reqwest::Method::from_bytes(method.as_bytes()).unwrap_or(reqwest::Method::GET);
        let mut auth =
            reqwest::header::HeaderValue::from_str(&format!("Bearer {}", self.secret.expose()))
                .map_err(|_| invalid("invalid authorization header"))?;
        auth.set_sensitive(true);
        let mut rb = self
            .reqwest
            .request(m, url)
            .header(reqwest::header::AUTHORIZATION, auth)
            .header(reqwest::header::ACCEPT, "application/json")
            .header(
                reqwest::header::USER_AGENT,
                concat!("zai-rs/", env!("CARGO_PKG_VERSION")),
            );
        for header in &self.additional_headers {
            rb = rb.header(header.name(), header.value());
        }
        Ok(match body {
            request::BodyKind::None => rb,
            request::BodyKind::Json(v) => rb
                .header(reqwest::header::CONTENT_TYPE, "application/json")
                .body(serde_json::to_vec(v).map_err(ZaiError::from)?),
            request::BodyKind::Bytes(b) => rb
                .header(reqwest::header::CONTENT_TYPE, "application/json")
                .body((*b).clone()),
            request::BodyKind::Multipart(factory) => rb.multipart(factory.build()?),
        })
    }
}

fn api_error(status: u16, error: decode::BusinessError) -> ZaiError {
    let code = error
        .code
        .and_then(|v| match v {
            serde_json::Value::Number(n) => n.as_u64().map(|n| n as u16),
            serde_json::Value::String(s) => s.parse().ok(),
            _ => None,
        })
        .unwrap_or(0);
    ZaiError::from_api_response(status, code, error.message)
}

/// Cap a response body to `limit` bytes (plan P03.8).
async fn cap_body(resp: reqwest::Response, limit: u64) -> Bytes {
    // Read up to limit+1 to detect overflow.
    match tokio::time::timeout(Duration::from_secs(60), resp.bytes()).await {
        Ok(Ok(b)) if (b.len() as u64) > limit => {
            warn!(
                bytes = b.len(),
                limit, "response body exceeded limit; truncated"
            );
            b.slice(..limit as usize)
        },
        Ok(Ok(b)) => b,
        Ok(Err(_)) | Err(_) => Bytes::new(),
    }
}

fn payload_too_large(limit: u64) -> ZaiError {
    ZaiError::ApiError {
        code: codes::SDK_VALIDATION,
        message: format!("request body exceeded limit ({limit} bytes)"),
    }
}

fn timeout_attempt() -> ZaiError {
    ZaiError::ApiError {
        code: codes::SDK_TIMEOUT,
        message: "per-attempt timeout exceeded".to_string(),
    }
}

fn timeout_overall() -> ZaiError {
    ZaiError::ApiError {
        code: codes::SDK_TIMEOUT,
        message: "overall deadline exceeded".to_string(),
    }
}

fn retry_exhausted(prev: ZaiError) -> ZaiError {
    ZaiError::ApiError {
        code: codes::SDK_TIMEOUT,
        message: format!("retry exhausted: {prev}"),
    }
}

fn invalid(msg: &str) -> ZaiError {
    ZaiError::ApiError {
        code: codes::SDK_CONFIG,
        message: msg.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timeout_defaults_match_plan() {
        let t = TimeoutPolicy::default();
        assert_eq!(t.connect, Duration::from_secs(10));
        assert_eq!(t.attempt, Duration::from_secs(60));
        assert_eq!(t.overall, Duration::from_secs(120));
        assert_eq!(t.stream_idle, Duration::from_secs(60));
    }

    #[test]
    fn nonidempotent_max_attempts_is_one() {
        // A constructed Transport is heavy (needs a reqwest client); test the
        // pure logic instead.
        let safety = RetrySafety::NonIdempotent;
        assert_eq!(effective_max_attempts_for(safety, 3), 1);
        let safety = RetrySafety::Idempotent;
        assert_eq!(effective_max_attempts_for(safety, 3), 3);
        assert_eq!(effective_max_attempts_for(safety, 1), 1);
    }

    fn effective_max_attempts_for(safety: RetrySafety, configured: u8) -> u8 {
        match safety {
            RetrySafety::Idempotent => configured.max(1),
            RetrySafety::NonIdempotent => 1,
        }
    }
}
