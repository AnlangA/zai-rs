//! HTTP transport used internally by [`ZaiClient`](super::ZaiClient).
//!
//! Execution pipeline:
//! build request → enforce the JSON request limit → send/retry → buffer and cap
//! the response → let the caller decode bytes or JSON.
//! File-content downloads use a separate pull-based byte stream so callers can
//! apply backpressure without buffering an entire file in memory.
//!
//! The default client uses a 10-second connect timeout and a 60-second
//! per-attempt timeout. A configured transport derives its overall deadline as
//! `request_timeout * max_attempts`. Backoff uses full jitter with an injectable
//! [`JitterSource`] so tests can use deterministic delays.
//!
//! JSON, empty-body, binary, and multipart endpoints share authentication,
//! retry, redirect, size-limit and response-decoding behavior.

pub mod decode;
pub mod download;
pub mod limits;
pub mod multipart;
pub mod redirect;
pub mod request;
pub mod retry;

use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use bytes::{Bytes, BytesMut};
use futures_util::{Stream, StreamExt};

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
use crate::client::transport::retry::{
    is_retryable_outcome, parse_retry_after, reconcile_retry_after,
};

/// Authenticated response bytes from a successful SSE request.
pub(crate) type SseByteStream = Pin<Box<dyn Stream<Item = ZaiResult<Bytes>> + Send + 'static>>;
/// Authenticated, bounded bytes from a successful file-content request.
pub(crate) type FileByteStream = Pin<Box<dyn Stream<Item = ZaiResult<Bytes>> + Send + 'static>>;
type ResponseByteStream =
    Pin<Box<dyn Stream<Item = Result<Bytes, reqwest::Error>> + Send + 'static>>;

/// Timeout values used by the transport.
///
/// `Default` yields a 60-second attempt and 120-second overall deadline. The
/// internal `Transport::new` constructor replaces `overall` with
/// `request_timeout * max_attempts` from the public transport configuration.
#[derive(Debug, Clone, Copy)]
pub struct TimeoutPolicy {
    /// Deadline for one HTTP attempt.
    pub attempt: Duration,
    /// Deadline covering attempts, redirects, and backoff.
    pub overall: Duration,
}

#[derive(Debug, Clone, Copy)]
struct RequestPolicy {
    attempt: Duration,
    overall: Duration,
    sse_handshake: Duration,
    sse_idle: Duration,
    max_attempts: u8,
}

impl Default for TimeoutPolicy {
    fn default() -> Self {
        Self {
            attempt: Duration::from_secs(60),
            overall: Duration::from_secs(120),
        }
    }
}

/// Shared HTTP client and immutable transport policy owned by one `ZaiClient`.
#[derive(Clone)]
pub(crate) struct Transport {
    pub(crate) reqwest: reqwest::Client,
    pub(crate) timeouts: TimeoutPolicy,
    pub(crate) max_attempts: u8,
    pub(crate) jitter: Arc<dyn JitterSource>,
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
    response_mode: ResponseMode,
}

impl TransportResponse {
    /// Consume the response and return its buffered body.
    ///
    /// A recognized business-error envelope or non-2xx HTTP status is converted
    /// to [`ZaiError`] before bytes are returned.
    pub fn bytes(self) -> ZaiResult<Bytes> {
        if let Some(error) = self.business_error() {
            return Err(api_error(self.status, error));
        }
        if !(200..300).contains(&self.status) {
            return Err(ZaiError::from_api_response(
                self.status,
                0,
                String::from_utf8_lossy(&self.body).into_owned(),
            ));
        }
        self.validate_success_content_type()?;
        Ok(self.body)
    }

    /// Consume the response and deserialize its buffered JSON body.
    ///
    /// Business-error envelopes and non-2xx statuses are handled before JSON
    /// deserialization. Successful responses must use the endpoint's
    /// documented JSON media type.
    pub fn json<T: serde::de::DeserializeOwned>(self) -> ZaiResult<T> {
        if let Some(error) = self.business_error() {
            return Err(api_error(self.status, error));
        }
        if !(200..300).contains(&self.status) {
            return Err(ZaiError::from_api_response(
                self.status,
                0,
                String::from_utf8_lossy(&self.body).into_owned(),
            ));
        }
        self.validate_success_content_type()?;
        serde_json::from_slice(&self.body).map_err(ZaiError::from)
    }

    fn validate_success_content_type(&self) -> ZaiResult<()> {
        let content_type = self
            .headers
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default();
        self.response_mode.validate_content_type(content_type)
    }

    fn business_error(&self) -> Option<decode::BusinessError> {
        let content_type = self
            .headers
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default();
        let should_probe = self.response_mode == ResponseMode::Json
            || !(200..300).contains(&self.status)
            || decode::validate_json_content_type(content_type).is_ok();
        should_probe
            .then(|| std::str::from_utf8(&self.body).ok())
            .flatten()
            .and_then(decode::extract_error_envelope)
    }
}

struct SystemJitter;
impl JitterSource for SystemJitter {
    fn jitter(&self, upper: Duration) -> Duration {
        let upper_nanos = u64::try_from(upper.as_nanos()).unwrap_or(u64::MAX);
        if upper_nanos == 0 {
            return Duration::ZERO;
        }
        Duration::from_nanos(fastrand::u64(0..=upper_nanos))
    }
}

/// Outcome of a single send attempt, used by the retry loop.
enum AttemptOutcome {
    /// A final HTTP response (status + headers + body bytes already capped).
    Response {
        status: u16,
        headers: reqwest::header::HeaderMap,
        body: Bytes,
    },
    /// A transient error that may be retried.
    Transient(ZaiError),
}

enum AttemptStep {
    Follow(url::Url),
    Outcome(AttemptOutcome),
    Final(TransportResponse),
}

struct FileStreamState {
    transport: Transport,
    method: &'static str,
    url: String,
    safety: RetrySafety,
    attempt: u8,
    max_attempts: u8,
    redirect_hops: u8,
    attempt_timeout: Duration,
    attempt_deadline: tokio::time::Instant,
    overall_deadline: tokio::time::Instant,
    body: Option<ResponseByteStream>,
    delivered: bool,
    total: u64,
    terminated: bool,
}

#[derive(Clone, Copy)]
struct AttemptContext {
    safety: RetrySafety,
    redirect_hops: u8,
    attempt_deadline: tokio::time::Instant,
    overall_deadline: tokio::time::Instant,
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
                attempt: config.request_timeout,
                overall: config
                    .request_timeout
                    .saturating_mul(u32::from(config.max_attempts)),
            },
            max_attempts: config.max_attempts,
            jitter: Arc::new(SystemJitter),
            secret,
            additional_headers: config.additional_headers.clone(),
        }
    }

    /// Send a prepared request with the retry/timeout pipeline, returning the
    /// (status, headers, body) of the final attempt. The caller performs the
    /// typed decode.
    ///
    /// Multipart bodies are rebuilt for every attempt. Responses are fully
    /// buffered; this method does not implement SSE streaming.
    #[tracing::instrument(
        name = "zai.http.request",
        skip_all,
        fields(
            operation_id = prepped.operation_id,
            method = prepped.method,
            route = prepped.route_template.as_str()
        )
    )]
    pub(crate) async fn send(&self, prepped: &PreparedRequest<'_>) -> ZaiResult<TransportResponse> {
        // Enforce request body limit up front.
        if let request::BodyKind::Bytes(b) = &prepped.body
            && (b.len() as u64) > JSON_REQUEST_MAX
        {
            return Err(payload_too_large(JSON_REQUEST_MAX));
        }

        let policy =
            resolve_request_policy(self.timeouts, self.max_attempts, prepped.request_options);
        let safety = prepped
            .retry_safety
            .effective(prepped.request_options.retry_override());
        let started = tokio::time::Instant::now();
        let deadline = started + policy.overall;
        let max_attempts = effective_max_attempts(safety, policy.max_attempts);

        let mut attempt: u8 = 1;
        let mut attempt_deadline = (started + policy.attempt).min(deadline);
        let mut url = prepped.url.clone();
        let mut hops: u8 = 0;
        loop {
            if tokio::time::Instant::now() >= deadline {
                return Err(timeout_overall());
            }

            let outcome = match self
                .perform_attempt(prepped, &url, safety, hops, attempt_deadline, deadline)
                .await?
            {
                AttemptStep::Follow(target) => {
                    hops += 1;
                    url = target.to_string();
                    // Redirects stay within the current attempt and do not
                    // consume the retry budget.
                    continue;
                },
                AttemptStep::Final(response) => return Ok(response),
                AttemptStep::Outcome(outcome) => outcome,
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
                        .and_then(|value| parse_business_code(&value));
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
                        if !delay_fits_before(delay, deadline) {
                            return Err(timeout_overall());
                        }
                        tokio::time::sleep(delay).await;
                        attempt += 1;
                        attempt_deadline =
                            (tokio::time::Instant::now() + policy.attempt).min(deadline);
                        continue;
                    }
                    return Ok(TransportResponse {
                        status,
                        headers,
                        body,
                        response_mode: prepped.response_mode,
                    });
                },
                AttemptOutcome::Transient(e) => {
                    if attempt >= max_attempts {
                        return Err(e);
                    }
                    // Network/timeout failures have no HTTP response headers, so
                    // this branch uses jitter without a Retry-After hint.
                    let delay = backoff_delay(u32::from(attempt) - 1, self.jitter.as_ref());
                    if !delay_fits_before(delay, deadline) {
                        return Err(timeout_overall());
                    }
                    tokio::time::sleep(delay).await;
                    attempt += 1;
                    attempt_deadline = (tokio::time::Instant::now() + policy.attempt).min(deadline);
                },
            }
        }
    }

    /// Start a pull-based file response without buffering its successful body.
    ///
    /// Authentication, redirects, retry classification, MIME/business-error
    /// validation and deadlines are completed inside the transport. A
    /// transient body failure may be retried only until the first byte chunk is
    /// yielded to the caller.
    #[tracing::instrument(
        name = "zai.http.file_stream",
        skip_all,
        fields(
            operation_id = prepped.operation_id,
            method = prepped.method,
            route = prepped.route_template.as_str()
        )
    )]
    pub(crate) async fn send_file(
        &self,
        prepped: &PreparedRequest<'_>,
    ) -> ZaiResult<FileByteStream> {
        if !matches!(&prepped.body, request::BodyKind::None)
            || prepped.response_mode != ResponseMode::File
        {
            return Err(invalid(
                "file streaming requires a body-less file response request",
            ));
        }

        let policy =
            resolve_request_policy(self.timeouts, self.max_attempts, prepped.request_options);
        let safety = prepped
            .retry_safety
            .effective(prepped.request_options.retry_override());
        let started = tokio::time::Instant::now();
        let overall_deadline = started + policy.overall;
        let mut state = FileStreamState {
            transport: self.clone(),
            method: prepped.method,
            url: prepped.url.clone(),
            safety,
            attempt: 1,
            max_attempts: effective_max_attempts(safety, policy.max_attempts),
            redirect_hops: 0,
            attempt_timeout: policy.attempt,
            attempt_deadline: (started + policy.attempt).min(overall_deadline),
            overall_deadline,
            body: None,
            delivered: false,
            total: 0,
            terminated: false,
        };
        state.open_response().await?;

        let stream = futures_util::stream::unfold(state, FileStreamState::next_item);
        Ok(Box::pin(stream))
    }

    /// Send one authenticated SSE request and return its response byte stream.
    ///
    /// Streaming POST requests are deliberately never retried or redirected:
    /// once the server has accepted a request, replaying it could duplicate a
    /// generation. The request/response handshake uses the configured attempt
    /// deadline; after that, each incoming chunk gets a fresh idle deadline.
    #[tracing::instrument(
        name = "zai.http.stream",
        skip_all,
        fields(
            operation_id = prepped.operation_id,
            method = prepped.method,
            route = prepped.route_template.as_str()
        )
    )]
    pub(crate) async fn send_sse(&self, prepped: &PreparedRequest<'_>) -> ZaiResult<SseByteStream> {
        if let request::BodyKind::Bytes(body) = &prepped.body
            && (body.len() as u64) > JSON_REQUEST_MAX
        {
            return Err(payload_too_large(JSON_REQUEST_MAX));
        }

        let policy =
            resolve_request_policy(self.timeouts, self.max_attempts, prepped.request_options);
        let deadline = tokio::time::Instant::now() + policy.sse_handshake;
        let request = tokio::time::timeout(
            policy.sse_handshake,
            self.build_request_with_accept(
                prepped.method,
                &prepped.url,
                &prepped.body,
                decode::SSE_CONTENT_TYPE,
            ),
        )
        .await
        .map_err(|_| timeout_sse_handshake())??;
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        let response = tokio::time::timeout(remaining, request.send())
            .await
            .map_err(|_| timeout_sse_handshake())?
            .map_err(ZaiError::from)?;

        let status = response.status().as_u16();
        let headers = response.headers().clone();
        let content_type = headers
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default();
        let content_type_error = decode::validate_sse_content_type(content_type).err();

        if !(200..300).contains(&status) || content_type_error.is_some() {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            let body = read_body(response, ERROR_BODY_MAX, remaining)
                .await
                .map_err(|error| sse_handshake_body_read_error(error, ERROR_BODY_MAX))?;
            if let Ok(text) = std::str::from_utf8(&body)
                && let Some(error) = decode::extract_error_envelope(text)
            {
                return Err(api_error(status, error));
            }
            if !(200..300).contains(&status) {
                return Err(ZaiError::from_api_response(
                    status,
                    0,
                    String::from_utf8_lossy(&body).into_owned(),
                ));
            }
            return Err(content_type_error
                .unwrap_or_else(|| invalid("SSE response did not use text/event-stream")));
        }

        let idle_timeout = policy.sse_idle;
        let byte_stream = response.bytes_stream();
        let idle_deadline = tokio::time::Instant::now() + idle_timeout;
        let stream = futures_util::stream::unfold(
            (Box::pin(byte_stream), false, idle_deadline),
            move |(mut inner, terminated, idle_deadline)| async move {
                if terminated {
                    return None;
                }
                // Prefer an already-buffered network chunk over an elapsed
                // timer. This keeps a paused consumer from manufacturing an
                // idle timeout when bytes arrived before the deadline, while
                // the absolute timer still fires immediately for a silent
                // server when polling resumes.
                let next = tokio::select! {
                    biased;
                    next = inner.next() => Some(next),
                    () = tokio::time::sleep_until(idle_deadline) => None,
                };
                match next {
                    Some(Some(Ok(chunk))) if (chunk.len() as u64) <= JSON_RESPONSE_MAX => {
                        let next_deadline = tokio::time::Instant::now() + idle_timeout;
                        Some((Ok(chunk), (inner, false, next_deadline)))
                    },
                    Some(Some(Ok(_))) => Some((
                        Err(response_too_large(JSON_RESPONSE_MAX)),
                        (inner, true, idle_deadline),
                    )),
                    Some(Some(Err(error))) => {
                        Some((Err(ZaiError::from(error)), (inner, true, idle_deadline)))
                    },
                    Some(None) => None,
                    None => Some((Err(timeout_sse_idle()), (inner, true, idle_deadline))),
                }
            },
        );
        Ok(Box::pin(stream))
    }

    async fn perform_attempt(
        &self,
        prepped: &PreparedRequest<'_>,
        url: &str,
        safety: RetrySafety,
        hops: u8,
        attempt_deadline: tokio::time::Instant,
        overall_deadline: tokio::time::Instant,
    ) -> ZaiResult<AttemptStep> {
        let started = tokio::time::Instant::now();
        let req = match tokio::time::timeout(
            attempt_deadline.saturating_duration_since(started),
            self.build_request(prepped.method, url, &prepped.body, prepped.response_mode),
        )
        .await
        {
            Ok(result) => result?,
            Err(_) => return timeout_step(overall_deadline),
        };
        let remaining = attempt_deadline.saturating_duration_since(tokio::time::Instant::now());
        let response = match tokio::time::timeout(remaining, req.send()).await {
            Err(_) => return timeout_step(overall_deadline),
            Ok(Err(error)) => {
                return Ok(AttemptStep::Outcome(AttemptOutcome::Transient(
                    error.into(),
                )));
            },
            Ok(Ok(response)) => response,
        };
        let context = AttemptContext {
            safety,
            attempt_deadline,
            overall_deadline,
            redirect_hops: hops,
        };
        self.classify_response(response, prepped, url, context)
            .await
    }

    async fn classify_response(
        &self,
        response: reqwest::Response,
        prepped: &PreparedRequest<'_>,
        current_url: &str,
        context: AttemptContext,
    ) -> ZaiResult<AttemptStep> {
        let status = response.status().as_u16();
        let headers = response.headers().clone();
        if (300..400).contains(&status) {
            let location = headers
                .get(reqwest::header::LOCATION)
                .and_then(|value| value.to_str().ok())
                .unwrap_or_default();
            let current = url::Url::parse(current_url).map_err(|_| invalid("current url parse"))?;
            if let Ok(Some(target)) = follow_redirect(
                &current,
                status,
                location,
                context.safety,
                prepped.method,
                context.redirect_hops,
            ) {
                return Ok(AttemptStep::Follow(target));
            }

            let remaining = context
                .attempt_deadline
                .saturating_duration_since(tokio::time::Instant::now());
            let body = read_body(response, ERROR_BODY_MAX, remaining)
                .await
                .map_err(|error| {
                    body_read_error(error, ERROR_BODY_MAX, context.overall_deadline)
                })?;
            return Ok(AttemptStep::Final(TransportResponse {
                status,
                headers,
                body,
                response_mode: prepped.response_mode,
            }));
        }

        let limit = response_limit(status, prepped.response_mode);
        let remaining = context
            .attempt_deadline
            .saturating_duration_since(tokio::time::Instant::now());
        let outcome = match read_body(response, limit, remaining).await {
            Ok(body) => AttemptOutcome::Response {
                status,
                headers,
                body,
            },
            Err(BodyReadError::Network(error)) => AttemptOutcome::Transient(error.into()),
            Err(BodyReadError::Timeout) => {
                if tokio::time::Instant::now() >= context.overall_deadline {
                    return Err(timeout_overall());
                }
                AttemptOutcome::Transient(timeout_attempt())
            },
            Err(BodyReadError::TooLarge) => return Err(response_too_large(limit)),
        };
        Ok(AttemptStep::Outcome(outcome))
    }

    async fn build_request(
        &self,
        method: &str,
        url: &str,
        body: &request::BodyKind<'_>,
        response_mode: ResponseMode,
    ) -> ZaiResult<reqwest::RequestBuilder> {
        self.build_request_with_accept(method, url, body, response_mode.accept())
            .await
    }

    async fn build_request_with_accept(
        &self,
        method: &str,
        url: &str,
        body: &request::BodyKind<'_>,
        accept: &'static str,
    ) -> ZaiResult<reqwest::RequestBuilder> {
        let m = reqwest::Method::from_bytes(method.as_bytes())
            .map_err(|_| invalid("invalid HTTP method"))?;
        let mut auth =
            reqwest::header::HeaderValue::from_str(&format!("Bearer {}", self.secret.expose()))
                .map_err(|_| invalid("invalid authorization header"))?;
        auth.set_sensitive(true);
        let mut rb = self
            .reqwest
            .request(m, url)
            .header(reqwest::header::AUTHORIZATION, auth)
            .header(reqwest::header::ACCEPT, accept)
            .header(
                reqwest::header::USER_AGENT,
                concat!("zai-rs/", env!("CARGO_PKG_VERSION")),
            );
        for header in &self.additional_headers {
            rb = rb.header(header.name(), header.value());
        }
        Ok(match body {
            request::BodyKind::None => rb,
            request::BodyKind::Bytes(b) => rb
                .header(reqwest::header::CONTENT_TYPE, "application/json")
                .body((*b).clone()),
            request::BodyKind::Multipart(factory) => rb.multipart(factory.build().await?),
        })
    }
}

impl FileStreamState {
    async fn open_response(&mut self) -> ZaiResult<()> {
        loop {
            if tokio::time::Instant::now() >= self.overall_deadline {
                return Err(timeout_overall());
            }

            let remaining = self
                .attempt_deadline
                .saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                let error = self.current_timeout();
                self.retry_transient(error).await?;
                continue;
            }

            let body = request::BodyKind::None;
            let request = match tokio::time::timeout(
                remaining,
                self.transport
                    .build_request(self.method, &self.url, &body, ResponseMode::File),
            )
            .await
            {
                Ok(result) => result?,
                Err(_) => {
                    let error = self.current_timeout();
                    self.retry_transient(error).await?;
                    continue;
                },
            };

            let remaining = self
                .attempt_deadline
                .saturating_duration_since(tokio::time::Instant::now());
            let response = match tokio::time::timeout(remaining, request.send()).await {
                Ok(Ok(response)) => response,
                Ok(Err(error)) => {
                    self.retry_transient(error.into()).await?;
                    continue;
                },
                Err(_) => {
                    let error = self.current_timeout();
                    self.retry_transient(error).await?;
                    continue;
                },
            };

            let status = response.status().as_u16();
            let headers = response.headers().clone();
            if (300..400).contains(&status) {
                let location = headers
                    .get(reqwest::header::LOCATION)
                    .and_then(|value| value.to_str().ok())
                    .unwrap_or_default();
                let current =
                    url::Url::parse(&self.url).map_err(|_| invalid("current url parse"))?;
                if let Ok(Some(target)) = follow_redirect(
                    &current,
                    status,
                    location,
                    self.safety,
                    self.method,
                    self.redirect_hops,
                ) {
                    self.redirect_hops += 1;
                    self.url = target.to_string();
                    continue;
                }
            }

            let content_type = headers
                .get(reqwest::header::CONTENT_TYPE)
                .and_then(|value| value.to_str().ok())
                .unwrap_or_default();
            let content_type_error = ResponseMode::File.validate_content_type(content_type).err();
            if (200..300).contains(&status) && content_type_error.is_none() {
                let announced_length = headers
                    .get(reqwest::header::CONTENT_LENGTH)
                    .and_then(|value| value.to_str().ok())
                    .and_then(|value| value.parse::<u64>().ok());
                if announced_length.is_some_and(|length| length > MULTIPART_FILE_BYTES_MAX) {
                    return Err(response_too_large(MULTIPART_FILE_BYTES_MAX));
                }
                self.body = Some(Box::pin(response.bytes_stream()));
                return Ok(());
            }

            let remaining = self
                .attempt_deadline
                .saturating_duration_since(tokio::time::Instant::now());
            let response_body = match read_body(response, ERROR_BODY_MAX, remaining).await {
                Ok(body) => body,
                Err(BodyReadError::Network(error)) => {
                    self.retry_transient(error.into()).await?;
                    continue;
                },
                Err(BodyReadError::Timeout) => {
                    let error = self.current_timeout();
                    self.retry_transient(error).await?;
                    continue;
                },
                Err(BodyReadError::TooLarge) => {
                    return Err(response_too_large(ERROR_BODY_MAX));
                },
            };

            let should_probe_business_error = !(200..300).contains(&status)
                || decode::validate_json_content_type(content_type).is_ok();
            let business_error = should_probe_business_error
                .then(|| std::str::from_utf8(&response_body).ok())
                .flatten()
                .and_then(decode::extract_error_envelope);
            let business_code = business_error
                .as_ref()
                .and_then(|error| error.code.as_ref())
                .and_then(parse_business_code);

            if self.safety == RetrySafety::Idempotent
                && is_retryable_outcome(status, business_code)
                && self.attempt < self.max_attempts
            {
                let retry_after = headers
                    .get(reqwest::header::RETRY_AFTER)
                    .and_then(|value| value.to_str().ok())
                    .and_then(parse_retry_after);
                self.schedule_retry(retry_after).await?;
                continue;
            }
            if let Some(error) = business_error {
                return Err(api_error(status, error));
            }
            if !(200..300).contains(&status) {
                return Err(ZaiError::from_api_response(
                    status,
                    0,
                    String::from_utf8_lossy(&response_body).into_owned(),
                ));
            }
            return Err(content_type_error
                .unwrap_or_else(|| invalid("file response did not use application/octet-stream")));
        }
    }

    async fn next_item(mut self) -> Option<(ZaiResult<Bytes>, Self)> {
        if self.terminated {
            return None;
        }

        loop {
            let remaining = self
                .attempt_deadline
                .saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                let error = self.current_timeout();
                if let Err(error) = self.retry_body_failure(error).await {
                    self.terminated = true;
                    self.body = None;
                    return Some((Err(error), self));
                }
                continue;
            }

            let next = match self.body.as_mut() {
                Some(body) => tokio::time::timeout(remaining, body.next()).await,
                None => {
                    self.terminated = true;
                    return Some((
                        Err(invalid("file response stream was not initialized")),
                        self,
                    ));
                },
            };
            match next {
                Ok(Some(Ok(chunk))) if chunk.is_empty() => continue,
                Ok(Some(Ok(chunk))) => {
                    let next_total = match self.total.checked_add(chunk.len() as u64) {
                        Some(total) if total <= MULTIPART_FILE_BYTES_MAX => total,
                        _ => {
                            self.terminated = true;
                            self.body = None;
                            return Some((Err(response_too_large(MULTIPART_FILE_BYTES_MAX)), self));
                        },
                    };
                    self.total = next_total;
                    self.delivered = true;
                    return Some((Ok(chunk), self));
                },
                Ok(Some(Err(error))) => {
                    if let Err(error) = self.retry_body_failure(error.into()).await {
                        self.terminated = true;
                        self.body = None;
                        return Some((Err(error), self));
                    }
                },
                Ok(None) => {
                    self.terminated = true;
                    self.body = None;
                    return None;
                },
                Err(_) => {
                    let error = self.current_timeout();
                    if let Err(error) = self.retry_body_failure(error).await {
                        self.terminated = true;
                        self.body = None;
                        return Some((Err(error), self));
                    }
                },
            }
        }
    }

    async fn retry_body_failure(&mut self, error: ZaiError) -> ZaiResult<()> {
        if self.delivered || self.attempt >= self.max_attempts {
            return Err(error);
        }
        self.body = None;
        self.schedule_retry(None).await?;
        self.open_response().await
    }

    async fn retry_transient(&mut self, error: ZaiError) -> ZaiResult<()> {
        if self.attempt >= self.max_attempts {
            return Err(error);
        }
        self.schedule_retry(None).await
    }

    async fn schedule_retry(&mut self, hint: Option<Duration>) -> ZaiResult<()> {
        let computed = backoff_delay(u32::from(self.attempt) - 1, self.transport.jitter.as_ref());
        let delay = reconcile_retry_after(hint, computed);
        if !delay_fits_before(delay, self.overall_deadline) {
            return Err(timeout_overall());
        }
        tokio::time::sleep(delay).await;
        if tokio::time::Instant::now() >= self.overall_deadline {
            return Err(timeout_overall());
        }
        self.attempt += 1;
        self.attempt_deadline =
            (tokio::time::Instant::now() + self.attempt_timeout).min(self.overall_deadline);
        Ok(())
    }

    fn current_timeout(&self) -> ZaiError {
        if tokio::time::Instant::now() >= self.overall_deadline {
            timeout_overall()
        } else {
            timeout_attempt()
        }
    }
}

fn effective_max_attempts(safety: RetrySafety, configured: u8) -> u8 {
    match safety {
        RetrySafety::Idempotent => configured.max(1),
        RetrySafety::NonIdempotent => 1,
    }
}

fn resolve_request_policy(
    defaults: TimeoutPolicy,
    configured_max_attempts: u8,
    options: crate::client::RequestOptions,
) -> RequestPolicy {
    let attempt = options.attempt_timeout().unwrap_or(defaults.attempt);
    let max_attempts = options
        .max_attempts()
        .unwrap_or(configured_max_attempts)
        .min(configured_max_attempts)
        .max(1);
    let overall = options.overall_timeout().unwrap_or_else(|| {
        if options.attempt_timeout().is_some() || options.max_attempts().is_some() {
            attempt.saturating_mul(u32::from(max_attempts))
        } else {
            defaults.overall
        }
    });
    let sse_handshake = options
        .sse_handshake_timeout()
        .unwrap_or(attempt)
        .min(overall);
    let sse_idle = options.sse_idle_timeout().unwrap_or(attempt);

    RequestPolicy {
        attempt,
        overall,
        sse_handshake,
        sse_idle,
        max_attempts,
    }
}

fn response_limit(status: u16, mode: ResponseMode) -> u64 {
    if !(200..300).contains(&status) {
        return ERROR_BODY_MAX;
    }
    match mode {
        ResponseMode::Json => JSON_RESPONSE_MAX,
        ResponseMode::File | ResponseMode::Audio => MULTIPART_FILE_BYTES_MAX,
    }
}

fn api_error(status: u16, error: decode::BusinessError) -> ZaiError {
    let Some(wire_code) = error.code.as_ref() else {
        return ZaiError::from_api_response(status, 0, error.message);
    };
    if let Some(code) = parse_business_code(wire_code) {
        let mapped = ZaiError::from_api_response(status, code, error.message.clone());
        if !matches!(&mapped, ZaiError::HttpBusinessError(_)) {
            return mapped;
        }
    }

    ZaiError::from_unrecognized_business_response(
        status,
        business_code_diagnostic(wire_code),
        error.message,
    )
}

pub(crate) fn parse_business_code(value: &serde_json::Value) -> Option<u16> {
    match value {
        serde_json::Value::Number(number) => number
            .as_u64()
            .and_then(|number| u16::try_from(number).ok()),
        serde_json::Value::String(number) => number.parse().ok(),
        _ => None,
    }
}

fn business_code_diagnostic(value: &serde_json::Value) -> String {
    const MAX_CHARS: usize = 128;

    fn bounded(value: &str) -> String {
        let mut chars = value.chars();
        let prefix: String = chars.by_ref().take(MAX_CHARS).collect();
        if chars.next().is_some() {
            format!("{prefix}…")
        } else {
            prefix
        }
    }

    match value {
        serde_json::Value::String(value) => {
            let value = bounded(&crate::client::error::mask_sensitive_info(value));
            serde_json::to_string(&value).unwrap_or_else(|_| "\"<text>\"".to_string())
        },
        serde_json::Value::Number(value) => bounded(&value.to_string()),
        serde_json::Value::Bool(value) => value.to_string(),
        serde_json::Value::Null => "null".to_string(),
        serde_json::Value::Array(_) => "<array>".to_string(),
        serde_json::Value::Object(_) => "<object>".to_string(),
    }
}

enum BodyReadError {
    Network(reqwest::Error),
    Timeout,
    TooLarge,
}

fn body_read_error(
    error: BodyReadError,
    limit: u64,
    overall_deadline: tokio::time::Instant,
) -> ZaiError {
    match error {
        BodyReadError::Network(error) => error.into(),
        BodyReadError::Timeout if tokio::time::Instant::now() >= overall_deadline => {
            timeout_overall()
        },
        BodyReadError::Timeout => timeout_attempt(),
        BodyReadError::TooLarge => response_too_large(limit),
    }
}

fn sse_handshake_body_read_error(error: BodyReadError, limit: u64) -> ZaiError {
    match error {
        BodyReadError::Network(error) => error.into(),
        BodyReadError::Timeout => timeout_sse_handshake(),
        BodyReadError::TooLarge => response_too_large(limit),
    }
}

fn timeout_step(overall_deadline: tokio::time::Instant) -> ZaiResult<AttemptStep> {
    if tokio::time::Instant::now() >= overall_deadline {
        Err(timeout_overall())
    } else {
        Ok(AttemptStep::Outcome(AttemptOutcome::Transient(
            timeout_attempt(),
        )))
    }
}

/// Stream a response into a bounded buffer. The limit is enforced while chunks
/// arrive, so an oversized or unbounded response is never fully allocated.
async fn read_body(
    resp: reqwest::Response,
    limit: u64,
    timeout: Duration,
) -> Result<Bytes, BodyReadError> {
    if timeout.is_zero() {
        return Err(BodyReadError::Timeout);
    }
    let read = async move {
        let initial_capacity = usize::try_from(limit.min(64 * 1024)).unwrap_or(64 * 1024);
        let mut body = BytesMut::with_capacity(initial_capacity);
        let mut stream = resp.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(BodyReadError::Network)?;
            let next_len = (body.len() as u64)
                .checked_add(chunk.len() as u64)
                .ok_or(BodyReadError::TooLarge)?;
            if next_len > limit {
                return Err(BodyReadError::TooLarge);
            }
            body.extend_from_slice(&chunk);
        }
        Ok(body.freeze())
    };
    tokio::time::timeout(timeout, read)
        .await
        .map_err(|_| BodyReadError::Timeout)?
}

fn payload_too_large(limit: u64) -> ZaiError {
    ZaiError::ApiError {
        code: codes::SDK_VALIDATION,
        message: format!("request body exceeded limit ({limit} bytes)"),
    }
}

fn response_too_large(limit: u64) -> ZaiError {
    ZaiError::ApiError {
        code: codes::SDK_VALIDATION,
        message: format!("response body exceeded limit ({limit} bytes)"),
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

fn timeout_sse_handshake() -> ZaiError {
    ZaiError::ApiError {
        code: codes::SDK_TIMEOUT,
        message: "SSE handshake timeout exceeded".to_string(),
    }
}

fn timeout_sse_idle() -> ZaiError {
    ZaiError::ApiError {
        code: codes::SDK_TIMEOUT,
        message: "SSE idle timeout exceeded".to_string(),
    }
}

fn delay_fits_before(delay: Duration, deadline: tokio::time::Instant) -> bool {
    deadline
        .checked_duration_since(tokio::time::Instant::now())
        .is_some_and(|remaining| delay < remaining)
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
        assert_eq!(t.attempt, Duration::from_secs(60));
        assert_eq!(t.overall, Duration::from_secs(120));
    }

    #[test]
    fn nonidempotent_max_attempts_is_one() {
        assert_eq!(effective_max_attempts(RetrySafety::NonIdempotent, 3), 1);
        assert_eq!(effective_max_attempts(RetrySafety::Idempotent, 3), 3);
        assert_eq!(effective_max_attempts(RetrySafety::Idempotent, 1), 1);
    }

    #[test]
    fn request_options_resolve_against_global_upper_bounds() {
        let defaults = TimeoutPolicy {
            attempt: Duration::from_secs(60),
            overall: Duration::from_secs(180),
        };
        let options = crate::client::RequestOptions::default()
            .with_attempt_timeout(Duration::from_secs(10))
            .unwrap()
            .with_sse_handshake_timeout(Duration::from_secs(4))
            .unwrap()
            .with_sse_idle_timeout(Duration::from_secs(7))
            .unwrap()
            .with_max_attempts(3)
            .unwrap();

        let policy = resolve_request_policy(defaults, 2, options);
        assert_eq!(policy.attempt, Duration::from_secs(10));
        assert_eq!(policy.overall, Duration::from_secs(20));
        assert_eq!(policy.sse_handshake, Duration::from_secs(4));
        assert_eq!(policy.sse_idle, Duration::from_secs(7));
        assert_eq!(policy.max_attempts, 2);

        let options = crate::client::RequestOptions::default()
            .with_attempt_timeout(Duration::from_secs(10))
            .unwrap()
            .with_overall_timeout(Duration::from_secs(3))
            .unwrap()
            .with_sse_handshake_timeout(Duration::from_secs(8))
            .unwrap();
        let policy = resolve_request_policy(defaults, 2, options);
        assert_eq!(policy.overall, Duration::from_secs(3));
        assert_eq!(policy.sse_handshake, Duration::from_secs(3));
    }

    #[test]
    fn system_jitter_respects_zero_and_upper_bound() {
        let jitter = SystemJitter;
        assert_eq!(jitter.jitter(Duration::ZERO), Duration::ZERO);
        let upper = Duration::from_millis(10);
        for _ in 0..100 {
            assert!(jitter.jitter(upper) <= upper);
        }
    }

    #[test]
    fn business_code_diagnostic_is_canonical_bounded_and_redacted() {
        assert_eq!(
            business_code_diagnostic(&serde_json::json!("UPSTREAM_BUSY")),
            r#""UPSTREAM_BUSY""#
        );
        assert_eq!(
            business_code_diagnostic(&serde_json::json!(70_000)),
            "70000"
        );

        let secret = "api_key=abc123.abcdefghijklmnopqrstuvwxyz";
        let diagnostic = business_code_diagnostic(&serde_json::json!(secret));
        assert!(!diagnostic.contains("abcdefghijklmnopqrstuvwxyz"));
        assert!(diagnostic.contains("[FILTERED]"));

        let diagnostic = business_code_diagnostic(&serde_json::json!("x".repeat(200)));
        assert!(diagnostic.chars().count() <= 131);
        assert!(diagnostic.ends_with("…\""));
    }
}
