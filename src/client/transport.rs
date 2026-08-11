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
use std::sync::{Arc, Mutex, MutexGuard};
use std::task::{Context, Poll};
use std::time::Duration;

use bytes::{Bytes, BytesMut};
use futures_util::task::AtomicWaker;
use futures_util::{Stream, StreamExt};
use tokio::sync::{Notify, OwnedSemaphorePermit, Semaphore, TryAcquireError};

use crate::ZaiError;
use crate::ZaiResult;
use crate::client::error::{RequestErrorMetadata, TimeoutPhase, codes};
use crate::client::secret::ApiSecret;
use crate::client::transport::limits::{
    ERROR_BODY_MAX, JSON_REQUEST_MAX, JSON_RESPONSE_MAX, MULTIPART_FILE_BYTES_MAX,
};
use crate::client::transport::redirect::follow as follow_redirect;
use crate::client::transport::request::{PreparedRequest, ResponseMode, SensitiveHeader};
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

const SSE_CONSUMER_IDLE_GRACE: Duration = Duration::from_secs(1);

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
    /// Proxy-free pool used exclusively for syntactic loopback destinations.
    direct_reqwest: Option<reqwest::Client>,
    pub(crate) timeouts: TimeoutPolicy,
    pub(crate) max_attempts: u8,
    pub(crate) jitter: Arc<dyn JitterSource>,
    /// Shared logical-operation bulkhead across all client clones.
    in_flight: Arc<Semaphore>,
    queue_timeout: Duration,
    stream_consumer_timeout: Duration,
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
    /// Result of the single transport-boundary business-envelope probe.
    ///
    /// Keeping the parsed error avoids reparsing a potentially large JSON
    /// success body when the caller consumes this response.
    business_probe: decode::ProbeOutcome,
    /// Business code derived alongside the single envelope probe for retry
    /// and final status fallback. This prevents a second bounded-prefix scan.
    business_code: Option<u16>,
    /// Whether a non-success diagnostic body reached EOF before its cap or
    /// deadline. Incomplete diagnostics are never parsed or surfaced.
    error_body_complete: bool,
    success_statuses: &'static [u16],
    response_mode: ResponseMode,
    attempts: u8,
    retry_after: Option<Duration>,
    in_flight_permit: Option<OwnedSemaphorePermit>,
}

impl TransportResponse {
    fn with_in_flight_permit(mut self, permit: OwnedSemaphorePermit) -> Self {
        self.in_flight_permit = Some(permit);
        self
    }

    /// Consume the response and return its buffered body.
    ///
    /// A recognized business-error envelope or non-2xx HTTP status is converted
    /// to [`ZaiError`] before bytes are returned.
    pub fn bytes(mut self) -> ZaiResult<Bytes> {
        self.validate_success_contract()
            .map_err(|error| self.annotate(error, None))?;
        match std::mem::take(&mut self.business_probe) {
            decode::ProbeOutcome::Error(error) => {
                let request_id = error.request_id.clone();
                return Err(self.annotate(api_error(self.status, error), request_id.as_deref()));
            },
            decode::ProbeOutcome::Ambiguous => {
                return Err(self.annotate(ambiguous_business_error(self.status), None));
            },
            decode::ProbeOutcome::Malformed if !(200..300).contains(&self.status) => {
                return Err(self.annotate(malformed_business_error(self.status), None));
            },
            decode::ProbeOutcome::Clean | decode::ProbeOutcome::Malformed => {},
        }
        if !(200..300).contains(&self.status) {
            if !self.error_body_complete {
                return Err(self.annotate(incomplete_error_body(self.status), None));
            }
            return Err(self.annotate(
                ZaiError::from_api_response(
                    self.status,
                    self.business_code.unwrap_or(0),
                    String::from_utf8_lossy(&self.body).into_owned(),
                ),
                None,
            ));
        }
        self.validate_success_content_type()
            .map_err(|error| self.annotate(error, None))?;
        Ok(self.body)
    }

    /// Consume the response and deserialize its buffered JSON body.
    ///
    /// Business-error envelopes and non-2xx statuses are handled before JSON
    /// deserialization. Successful responses must use the endpoint's
    /// documented JSON media type.
    pub fn json<T: serde::de::DeserializeOwned>(mut self) -> ZaiResult<T> {
        self.validate_success_contract()
            .map_err(|error| self.annotate(error, None))?;
        match std::mem::take(&mut self.business_probe) {
            decode::ProbeOutcome::Error(error) => {
                let request_id = error.request_id.clone();
                return Err(self.annotate(api_error(self.status, error), request_id.as_deref()));
            },
            decode::ProbeOutcome::Ambiguous => {
                return Err(self.annotate(ambiguous_business_error(self.status), None));
            },
            decode::ProbeOutcome::Malformed if !(200..300).contains(&self.status) => {
                return Err(self.annotate(malformed_business_error(self.status), None));
            },
            decode::ProbeOutcome::Clean | decode::ProbeOutcome::Malformed => {},
        }
        if !(200..300).contains(&self.status) {
            if !self.error_body_complete {
                return Err(self.annotate(incomplete_error_body(self.status), None));
            }
            return Err(self.annotate(
                ZaiError::from_api_response(
                    self.status,
                    self.business_code.unwrap_or(0),
                    String::from_utf8_lossy(&self.body).into_owned(),
                ),
                None,
            ));
        }
        self.validate_success_content_type()
            .map_err(|error| self.annotate(error, None))?;
        serde_json::from_slice(&self.body)
            .map_err(ZaiError::from)
            .map_err(|error| self.annotate(error, None))
    }

    fn validate_success_content_type(&self) -> ZaiResult<()> {
        let content_type = self
            .headers
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default();
        self.response_mode.validate_content_type(content_type)
    }

    fn validate_success_contract(&self) -> ZaiResult<()> {
        if !(200..300).contains(&self.status) {
            return Ok(());
        }
        if !self.success_statuses.contains(&self.status) {
            return Err(invalid_response(
                "response used an undocumented HTTP success status",
            ));
        }
        if self.headers.contains_key(reqwest::header::CONTENT_RANGE) {
            return Err(invalid_response(
                "complete response unexpectedly included Content-Range",
            ));
        }
        Ok(())
    }

    fn annotate(&self, error: ZaiError, body_request_id: Option<&str>) -> ZaiError {
        let request_id =
            sanitize_request_id(body_request_id).or_else(|| request_id_from_headers(&self.headers));
        error.with_request_metadata(
            RequestErrorMetadata::for_attempts(self.attempts)
                .with_request_id(request_id)
                .with_retry_after(self.retry_after),
        )
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
        error_body_complete: bool,
    },
    /// A transient error that may be retried.
    Transient(ZaiError),
}

enum AttemptStep {
    Follow {
        target: url::Url,
        request_id: Option<String>,
        retry_after: Option<Duration>,
    },
    Reject {
        error: ZaiError,
        request_id: Option<String>,
        retry_after: Option<Duration>,
    },
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
    request_id: Option<String>,
    retry_after: Option<Duration>,
    lease_expiry_error: Arc<Mutex<ZaiError>>,
    lease_fail_closed_error: Arc<Mutex<ZaiError>>,
    in_flight_permit: Option<OwnedSemaphorePermit>,
    sensitive_headers: Vec<SensitiveHeader>,
}

struct SseStreamState {
    inner: Option<ResponseByteStream>,
    terminated: bool,
    idle_deadline: tokio::time::Instant,
    idle_timeout: Duration,
    metadata: RequestErrorMetadata,
    in_flight_permit: Option<OwnedSemaphorePermit>,
}

/// Shared ownership slot for a raw response stream and its admission permit.
///
/// `inner` is always taken and dropped as one value. The wrapped stream owns
/// both the live response body and its `OwnedSemaphorePermit`, so no expiry or
/// cleanup path can release admission while leaving the HTTP response alive.
struct WholeStreamLeaseState {
    inner: Option<SseByteStream>,
    deadline: tokio::time::Instant,
    sliding_timeout: Option<Duration>,
    expiry_error: WholeStreamErrorSource,
    fail_closed_error: WholeStreamErrorSource,
    terminal_error: Option<ZaiError>,
    finished: bool,
}

struct WholeStreamLeaseShared {
    state: Mutex<WholeStreamLeaseState>,
    consumer_waker: AtomicWaker,
    deadline_changed: Notify,
}

struct WholeStreamLease {
    shared: Arc<WholeStreamLeaseShared>,
    watchdog: tokio::task::JoinHandle<()>,
}

/// Drop guard owned by the watchdog future from before its first poll.
///
/// Tokio drops the future when a task is aborted or its runtime shuts down.
/// Keeping this guard inside that future makes those paths synchronously close
/// the complete raw stream slot as well.
struct WholeStreamWatchdogGuard {
    shared: Arc<WholeStreamLeaseShared>,
}

#[derive(Clone)]
enum WholeStreamErrorSource {
    Fixed(ZaiError),
    Shared(Arc<Mutex<ZaiError>>),
}

impl WholeStreamErrorSource {
    fn current(&self) -> ZaiError {
        match self {
            Self::Fixed(error) => error.clone(),
            Self::Shared(slot) => match slot.lock() {
                Ok(error) => error.clone(),
                Err(poisoned) => {
                    let error = poisoned.into_inner().clone();
                    slot.clear_poison();
                    error
                },
            },
        }
    }
}

impl WholeStreamLeaseState {
    fn drop_inner(&mut self) {
        if let Some(inner) = self.inner.take() {
            // `inner` owns both the response body and its semaphore permit.
            // Drop it as one value while the shared slot is locked, so another
            // close path cannot observe or split that ownership.
            drop(inner);
        }
    }

    fn expire(&mut self) {
        if self.inner.is_none() {
            return;
        }
        self.terminal_error = Some(self.expiry_error.current());
        self.drop_inner();
    }

    fn fail_closed(&mut self) -> bool {
        if self.inner.is_none() {
            if !self.finished && self.terminal_error.is_none() {
                self.terminal_error = Some(self.fail_closed_error.current());
                return true;
            }
            return false;
        }
        self.terminal_error = Some(self.fail_closed_error.current());
        self.drop_inner();
        true
    }

    fn finish(&mut self) {
        self.drop_inner();
        self.terminal_error = None;
        self.finished = true;
    }

    fn take_terminal_error(&mut self) -> Option<ZaiError> {
        let error = self.terminal_error.take()?;
        self.finished = true;
        Some(error)
    }
}

impl WholeStreamLeaseShared {
    fn new(
        inner: SseByteStream,
        deadline: tokio::time::Instant,
        sliding_timeout: Option<Duration>,
        expiry_error: WholeStreamErrorSource,
        fail_closed_error: WholeStreamErrorSource,
    ) -> Arc<Self> {
        Arc::new(Self {
            state: Mutex::new(WholeStreamLeaseState {
                inner: Some(inner),
                deadline,
                sliding_timeout,
                expiry_error,
                fail_closed_error,
                terminal_error: None,
                finished: false,
            }),
            consumer_waker: AtomicWaker::new(),
            deadline_changed: Notify::new(),
        })
    }

    /// Recovering a poisoned slot always closes the complete inner stream and
    /// converts the panic boundary into one safe terminal error.
    fn lock_state(&self) -> MutexGuard<'_, WholeStreamLeaseState> {
        match self.state.lock() {
            Ok(state) => state,
            Err(poisoned) => {
                let mut state = poisoned.into_inner();
                state.fail_closed();
                self.state.clear_poison();
                state
            },
        }
    }

    fn poll_inner(&self, cx: &mut Context<'_>) -> Poll<Option<ZaiResult<Bytes>>> {
        // Register before inspecting state. If the watchdog wins immediately
        // afterwards, either this poll observes its terminal error or the wake
        // is retained by `AtomicWaker` for the executor.
        self.consumer_waker.register(cx.waker());
        let mut state = self.lock_state();

        if let Some(error) = state.take_terminal_error() {
            drop(state);
            drop(self.consumer_waker.take());
            self.deadline_changed.notify_one();
            return Poll::Ready(Some(Err(error)));
        }
        if state.finished || state.inner.is_none() {
            state.finish();
            drop(state);
            drop(self.consumer_waker.take());
            self.deadline_changed.notify_one();
            return Poll::Ready(None);
        }

        if tokio::time::Instant::now() >= state.deadline {
            state.expire();
            let error = state
                .take_terminal_error()
                .expect("an open expired lease must produce one terminal error");
            drop(state);
            drop(self.consumer_waker.take());
            self.deadline_changed.notify_one();
            return Poll::Ready(Some(Err(error)));
        }

        let polled = state
            .inner
            .as_mut()
            .expect("open lease must retain its complete inner stream")
            .as_mut()
            .poll_next(cx);

        // Polling an arbitrary Stream is synchronous, but it may spend enough
        // CPU time to cross the deadline. Recheck before yielding any chunk so
        // the watchdog and consumer sides implement the same boundary.
        let now = tokio::time::Instant::now();
        if now >= state.deadline {
            state.expire();
            let error = state
                .take_terminal_error()
                .expect("an open expired lease must produce one terminal error");
            drop(state);
            drop(self.consumer_waker.take());
            self.deadline_changed.notify_one();
            return Poll::Ready(Some(Err(error)));
        }

        match polled {
            Poll::Ready(Some(Ok(chunk))) => {
                let reset = if let Some(timeout) = state.sliding_timeout {
                    state.deadline = now + timeout;
                    // This measures raw-byte-stream progress. A typed SSE
                    // parser may consume several buffered events before it
                    // polls this raw stream again.
                    true
                } else {
                    false
                };
                drop(state);
                drop(self.consumer_waker.take());
                if reset {
                    self.deadline_changed.notify_one();
                }
                Poll::Ready(Some(Ok(chunk)))
            },
            Poll::Ready(Some(Err(error))) => {
                state.finish();
                drop(state);
                drop(self.consumer_waker.take());
                self.deadline_changed.notify_one();
                Poll::Ready(Some(Err(error)))
            },
            Poll::Ready(None) => {
                state.finish();
                drop(state);
                drop(self.consumer_waker.take());
                self.deadline_changed.notify_one();
                Poll::Ready(None)
            },
            Poll::Pending => Poll::Pending,
        }
    }

    fn open_deadline(&self) -> Option<tokio::time::Instant> {
        let state = self.lock_state();
        let should_wake =
            state.inner.is_none() && state.terminal_error.is_some() && !state.finished;
        let deadline = (state.inner.is_some() && !state.finished).then_some(state.deadline);
        drop(state);
        if should_wake {
            self.consumer_waker.wake();
        }
        deadline
    }

    fn expire_if_due(&self, now: tokio::time::Instant) -> bool {
        let mut state = self.lock_state();
        if state.inner.is_none() || state.finished || now < state.deadline {
            return false;
        }
        state.expire();
        true
    }

    fn close_by_consumer(&self) {
        let mut state = self.lock_state();
        state.finish();
        drop(state);
        drop(self.consumer_waker.take());
        self.deadline_changed.notify_one();
    }

    fn fail_closed_if_open(&self) -> bool {
        let mut state = self.lock_state();
        let should_wake = state.fail_closed()
            || (state.inner.is_none() && state.terminal_error.is_some() && !state.finished);
        drop(state);
        if should_wake {
            self.deadline_changed.notify_one();
        }
        should_wake
    }
}

impl WholeStreamLease {
    fn new(
        inner: SseByteStream,
        deadline: tokio::time::Instant,
        sliding_timeout: Option<Duration>,
        expiry_error: WholeStreamErrorSource,
        fail_closed_error: WholeStreamErrorSource,
    ) -> ZaiResult<Self> {
        let construction_error = fail_closed_error.current();
        let shared = WholeStreamLeaseShared::new(
            inner,
            deadline,
            sliding_timeout,
            expiry_error,
            fail_closed_error,
        );
        let guard = WholeStreamWatchdogGuard {
            shared: Arc::clone(&shared),
        };
        let handle = match tokio::runtime::Handle::try_current() {
            Ok(handle) => handle,
            Err(_) => {
                // Do not let a convenience constructor panic after the HTTP
                // response has already been acquired. Closing the shared slot
                // synchronously drops its body and admission permit together.
                shared.close_by_consumer();
                return Err(construction_error);
            },
        };
        // `guard` is moved into the future before spawning. Even if the task is
        // canceled before its first poll, dropping the future closes `inner`.
        let watchdog = handle.spawn(guard.run());
        Ok(Self { shared, watchdog })
    }

    fn boxed(
        inner: SseByteStream,
        deadline: tokio::time::Instant,
        sliding_timeout: Option<Duration>,
        expiry_error: WholeStreamErrorSource,
        fail_closed_error: WholeStreamErrorSource,
    ) -> ZaiResult<SseByteStream> {
        Ok(Box::pin(Self::new(
            inner,
            deadline,
            sliding_timeout,
            expiry_error,
            fail_closed_error,
        )?))
    }
}

impl Stream for WholeStreamLease {
    type Item = ZaiResult<Bytes>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.get_mut().shared.poll_inner(cx)
    }
}

impl Drop for WholeStreamLease {
    fn drop(&mut self) {
        // Close synchronously before requesting task cancellation. The
        // watchdog guard then observes a finished slot and becomes a no-op.
        self.shared.close_by_consumer();
        self.watchdog.abort();
    }
}

impl WholeStreamWatchdogGuard {
    async fn run(self) {
        loop {
            // Create the notification future before reading the generation's
            // deadline. `Notify` then retains a concurrent reset without a
            // lost-wake window.
            let changed = self.shared.deadline_changed.notified();
            tokio::pin!(changed);
            let Some(deadline) = self.shared.open_deadline() else {
                return;
            };

            tokio::select! {
                _ = tokio::time::sleep_until(deadline) => {
                    if self.shared.expire_if_due(tokio::time::Instant::now()) {
                        // `expire_if_due` drops the body+permit before waking.
                        self.shared.consumer_waker.wake();
                        return;
                    }
                },
                () = &mut changed => {},
            }
        }
    }
}

impl Drop for WholeStreamWatchdogGuard {
    fn drop(&mut self) {
        if self.shared.fail_closed_if_open() {
            // Runtime shutdown, task abort, and task panic all arrive here.
            // Wake only after the complete raw stream has been dropped.
            self.shared.consumer_waker.wake();
        }
    }
}

#[derive(Clone, Copy)]
struct AttemptContext {
    safety: RetrySafety,
    attempt: u8,
    redirect_hops: u8,
    attempt_deadline: tokio::time::Instant,
    overall_deadline: tokio::time::Instant,
}

impl Transport {
    pub(crate) fn new(
        reqwest: reqwest::Client,
        direct_reqwest: Option<reqwest::Client>,
        secret: ApiSecret,
        config: &crate::client::HttpTransportConfig,
        concurrency: &crate::client::HttpConcurrencyConfig,
    ) -> Self {
        Self {
            reqwest,
            direct_reqwest,
            timeouts: TimeoutPolicy {
                attempt: config.request_timeout,
                overall: config
                    .request_timeout
                    .saturating_mul(u32::from(config.max_attempts)),
            },
            max_attempts: config.max_attempts,
            jitter: Arc::new(SystemJitter),
            in_flight: Arc::new(Semaphore::new(concurrency.max_in_flight())),
            queue_timeout: concurrency.queue_timeout(),
            stream_consumer_timeout: concurrency.stream_consumer_timeout(),
            secret,
            additional_headers: config.additional_headers.clone(),
        }
    }

    async fn acquire_in_flight(
        &self,
        options: crate::client::RequestOptions,
    ) -> ZaiResult<OwnedSemaphorePermit> {
        let queue_timeout = options
            .queue_timeout()
            .map(|requested| requested.min(self.queue_timeout))
            .unwrap_or(self.queue_timeout);
        let semaphore = Arc::clone(&self.in_flight);

        if queue_timeout.is_zero() {
            return match semaphore.try_acquire_owned() {
                Ok(permit) => Ok(permit),
                Err(TryAcquireError::NoPermits) => Err(timeout_queue()),
                Err(TryAcquireError::Closed) => Err(concurrency_limiter_closed()),
            };
        }

        match tokio::time::timeout(queue_timeout, semaphore.acquire_owned()).await {
            Ok(Ok(permit)) => Ok(permit),
            Ok(Err(_)) => Err(concurrency_limiter_closed()),
            Err(_) => Err(timeout_queue()),
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
        let in_flight_permit = self.acquire_in_flight(prepped.request_options).await?;

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
        let mut last_retry_after = None;
        let mut last_request_id = None;
        loop {
            if tokio::time::Instant::now() >= deadline {
                return Err(annotate_request_error(
                    timeout_overall(),
                    attempt.saturating_sub(1).max(1),
                    last_request_id,
                    last_retry_after,
                ));
            }

            let context = AttemptContext {
                safety,
                attempt,
                redirect_hops: hops,
                attempt_deadline,
                overall_deadline: deadline,
            };
            let outcome =
                match self
                    .perform_attempt(prepped, &url, context)
                    .await
                    .map_err(|error| {
                        annotate_request_error(
                            error,
                            attempt,
                            last_request_id.clone(),
                            last_retry_after,
                        )
                    })? {
                    AttemptStep::Follow {
                        target,
                        request_id,
                        retry_after,
                    } => {
                        // If the redirected hop fails before producing another
                        // response, diagnostics should describe the most
                        // recent HTTP response rather than an earlier retry.
                        last_request_id = request_id;
                        last_retry_after = retry_after;
                        hops += 1;
                        url = target.to_string();
                        // Redirects stay within the current attempt and do not
                        // consume the retry budget.
                        continue;
                    },
                    AttemptStep::Reject {
                        error,
                        request_id,
                        retry_after,
                    } => {
                        return Err(annotate_request_error(
                            error,
                            attempt,
                            request_id,
                            retry_after,
                        ));
                    },
                    AttemptStep::Final(response) => {
                        return Ok(response.with_in_flight_permit(in_flight_permit));
                    },
                    AttemptStep::Outcome(outcome) => outcome,
                };

            match outcome {
                AttemptOutcome::Response {
                    status,
                    headers,
                    body,
                    error_body_complete,
                } => {
                    let should_probe =
                        should_probe_business_error(status, prepped.response_mode, &headers);
                    let business_probe = if error_body_complete {
                        probe_business_response(should_probe, &body)
                    } else {
                        decode::ProbeOutcome::Clean
                    };
                    let business_code = error_body_complete
                        .then(|| business_code_from_probe(&business_probe))
                        .flatten();
                    let retry_after = retry_after_from_headers(&headers);
                    let request_id = request_id_from_probe(&business_probe)
                        .or_else(|| request_id_from_headers(&headers));
                    if safety == RetrySafety::Idempotent
                        && is_retryable_outcome(status, business_code)
                        && attempt < max_attempts
                    {
                        let computed = backoff_delay(u32::from(attempt) - 1, self.jitter.as_ref());
                        let delay = reconcile_retry_after(retry_after, computed);
                        if !delay_fits_before(delay, deadline) {
                            return Err(annotate_request_error(
                                timeout_overall(),
                                attempt,
                                request_id,
                                retry_after,
                            ));
                        }
                        last_retry_after = retry_after;
                        last_request_id = request_id;
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
                        business_probe,
                        business_code,
                        error_body_complete,
                        success_statuses: prepped.success_statuses,
                        response_mode: prepped.response_mode,
                        attempts: attempt,
                        retry_after,
                        in_flight_permit: Some(in_flight_permit),
                    });
                },
                AttemptOutcome::Transient(e) => {
                    if attempt >= max_attempts {
                        return Err(annotate_request_error(
                            e,
                            attempt,
                            last_request_id,
                            last_retry_after,
                        ));
                    }
                    // Network/timeout failures have no HTTP response headers, so
                    // this branch uses jitter without a Retry-After hint.
                    let delay = backoff_delay(u32::from(attempt) - 1, self.jitter.as_ref());
                    if !delay_fits_before(delay, deadline) {
                        return Err(annotate_request_error(
                            timeout_overall(),
                            attempt,
                            last_request_id,
                            last_retry_after,
                        ));
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
        let in_flight_permit = self.acquire_in_flight(prepped.request_options).await?;

        let policy =
            resolve_request_policy(self.timeouts, self.max_attempts, prepped.request_options);
        let safety = prepped
            .retry_safety
            .effective(prepped.request_options.retry_override());
        let started = tokio::time::Instant::now();
        let overall_deadline = started + policy.overall;
        let lease_expiry_error = Arc::new(Mutex::new(annotate_request_error(
            timeout_overall(),
            1,
            None,
            None,
        )));
        let lease_fail_closed_error = Arc::new(Mutex::new(annotate_request_error(
            stream_lease_unavailable(),
            1,
            None,
            None,
        )));
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
            request_id: None,
            retry_after: None,
            lease_expiry_error: Arc::clone(&lease_expiry_error),
            lease_fail_closed_error: Arc::clone(&lease_fail_closed_error),
            in_flight_permit: Some(in_flight_permit),
            sensitive_headers: prepped.sensitive_headers.clone(),
        };
        if let Err(error) = state.open_response().await {
            return Err(state.annotate(error));
        }

        let lease_deadline = state.overall_deadline;
        let stream: FileByteStream = Box::pin(futures_util::stream::unfold(
            state,
            FileStreamState::next_item,
        ));
        WholeStreamLease::boxed(
            stream,
            lease_deadline,
            None,
            WholeStreamErrorSource::Shared(lease_expiry_error),
            WholeStreamErrorSource::Shared(lease_fail_closed_error),
        )
    }

    /// Send one authenticated SSE request and return its response byte stream.
    ///
    /// Streaming POST requests are deliberately never retried or redirected:
    /// once the server has accepted a request, replaying it could duplicate a
    /// generation. The request/response handshake uses the configured attempt
    /// deadline and succeeds only for an unranged `200 OK` event stream; after
    /// that, each incoming chunk gets a fresh idle deadline.
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
        let in_flight_permit = self.acquire_in_flight(prepped.request_options).await?;

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
                &prepped.sensitive_headers,
            ),
        )
        .await
        .map_err(|_| annotate_request_error(timeout_sse_handshake(), 1, None, None))?
        .map_err(|error| annotate_request_error(error, 1, None, None))?;
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        let response = tokio::time::timeout(remaining, request.send())
            .await
            .map_err(|_| annotate_request_error(timeout_sse_handshake(), 1, None, None))?
            .map_err(ZaiError::from)
            .map_err(|error| annotate_request_error(error, 1, None, None))?;

        let status = response.status().as_u16();
        let headers = response.headers().clone();
        let header_request_id = omit_sensitive_request_id(
            request_id_from_headers(&headers),
            &prepped.sensitive_headers,
        );
        let retry_after = retry_after_from_headers_omitting_sensitive_values(
            &headers,
            &prepped.sensitive_headers,
        );
        let content_type = headers
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default();
        let content_type_error = decode::validate_sse_content_type(content_type).err();
        let has_content_range = headers.contains_key(reqwest::header::CONTENT_RANGE);

        if (200..300).contains(&status)
            && (status != 200 || has_content_range || content_type_error.is_some())
        {
            let protocol_error = if status != 200 {
                invalid_response("SSE response requires HTTP 200 OK")
            } else if has_content_range {
                invalid_response("SSE response unexpectedly included Content-Range")
            } else {
                content_type_error.unwrap_or_else(|| {
                    invalid_response("SSE response did not use text/event-stream")
                })
            };
            return Err(annotate_request_error(
                protocol_error,
                1,
                header_request_id,
                retry_after,
            ));
        }

        if !(200..300).contains(&status) {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            // Error bodies are diagnostics, not the source of HTTP recovery
            // semantics. Keep only a bounded prefix so an oversized/streaming
            // error page cannot hide a known 401/429/5xx status behind an SDK
            // size or body-read error.
            let body = read_error_body(response, ERROR_BODY_MAX, remaining).await;
            if !body.complete {
                return Err(annotate_request_error(
                    incomplete_error_body(status),
                    1,
                    header_request_id,
                    retry_after,
                ));
            }
            let business_probe = probe_business_response(true, &body.bytes);
            match business_probe {
                decode::ProbeOutcome::Error(mut error) => {
                    error.message =
                        redact_sensitive_header_values(&error.message, &prepped.sensitive_headers);
                    error.code = error.code.filter(|code| {
                        !business_code_contains_sensitive_header_value(
                            code,
                            &prepped.sensitive_headers,
                        )
                    });
                    let request_id = omit_sensitive_request_id(
                        sanitize_request_id(error.request_id.as_deref()),
                        &prepped.sensitive_headers,
                    )
                    .or_else(|| header_request_id.clone());
                    return Err(annotate_request_error(
                        api_error(status, error),
                        1,
                        request_id,
                        retry_after,
                    ));
                },
                decode::ProbeOutcome::Ambiguous => {
                    return Err(annotate_request_error(
                        ambiguous_business_error(status),
                        1,
                        header_request_id,
                        retry_after,
                    ));
                },
                decode::ProbeOutcome::Malformed => {
                    return Err(annotate_request_error(
                        malformed_business_error(status),
                        1,
                        header_request_id,
                        retry_after,
                    ));
                },
                decode::ProbeOutcome::Clean => {},
            }
            let message = redact_sensitive_header_values(
                &String::from_utf8_lossy(&body.bytes),
                &prepped.sensitive_headers,
            );
            return Err(annotate_request_error(
                ZaiError::from_api_response(status, 0, message),
                1,
                header_request_id,
                retry_after,
            ));
        }

        let idle_timeout = policy.sse_idle;
        let stream_metadata = RequestErrorMetadata::for_attempts(1)
            .with_request_id(header_request_id)
            .with_retry_after(retry_after);
        let state = SseStreamState {
            inner: Some(Box::pin(response.bytes_stream())),
            terminated: false,
            idle_deadline: tokio::time::Instant::now() + idle_timeout,
            idle_timeout,
            metadata: stream_metadata.clone(),
            in_flight_permit: Some(in_flight_permit),
        };
        // The consumer lease is deliberately later than the upstream-silence
        // deadline. A caller actively awaiting data must observe `SseIdle`,
        // while a retained but unpolled raw stream is eventually reaped.
        let consumer_timeout = effective_stream_consumer_timeout(
            self.stream_consumer_timeout,
            prepped.request_options.stream_consumer_timeout(),
            idle_timeout,
        );
        let lease_deadline = tokio::time::Instant::now() + consumer_timeout;
        let expiry_error = timeout_stream_consumer().with_request_metadata(stream_metadata.clone());
        let fail_closed_error = stream_lease_unavailable().with_request_metadata(stream_metadata);
        let stream: SseByteStream = Box::pin(futures_util::stream::unfold(
            state,
            SseStreamState::next_item,
        ));
        WholeStreamLease::boxed(
            stream,
            lease_deadline,
            Some(consumer_timeout),
            WholeStreamErrorSource::Fixed(expiry_error),
            WholeStreamErrorSource::Fixed(fail_closed_error),
        )
    }

    async fn perform_attempt(
        &self,
        prepped: &PreparedRequest<'_>,
        url: &str,
        context: AttemptContext,
    ) -> ZaiResult<AttemptStep> {
        let started = tokio::time::Instant::now();
        let req = match tokio::time::timeout(
            context.attempt_deadline.saturating_duration_since(started),
            self.build_request(
                prepped.method,
                url,
                &prepped.body,
                prepped.response_mode,
                &prepped.sensitive_headers,
            ),
        )
        .await
        {
            Ok(result) => result?,
            Err(_) => return timeout_step(context.overall_deadline),
        };
        let remaining = context
            .attempt_deadline
            .saturating_duration_since(tokio::time::Instant::now());
        let response = match tokio::time::timeout(remaining, req.send()).await {
            Err(_) => return timeout_step(context.overall_deadline),
            Ok(Err(error)) => {
                return Ok(AttemptStep::Outcome(AttemptOutcome::Transient(
                    error.into(),
                )));
            },
            Ok(Ok(response)) => response,
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
            let location = match redirect_location(&headers) {
                Ok(location) => location,
                Err(error) => {
                    return Ok(AttemptStep::Reject {
                        error,
                        request_id: request_id_from_headers(&headers),
                        retry_after: retry_after_from_headers(&headers),
                    });
                },
            };
            let current = url::Url::parse(current_url).map_err(|_| invalid("current url parse"))?;
            match follow_redirect(
                &current,
                status,
                location,
                context.safety,
                prepped.method,
                context.redirect_hops,
            ) {
                Ok(Some(target)) => {
                    return Ok(AttemptStep::Follow {
                        target,
                        request_id: request_id_from_headers(&headers),
                        retry_after: retry_after_from_headers(&headers),
                    });
                },
                Ok(None) => {},
                Err(error) => {
                    return Ok(AttemptStep::Reject {
                        error,
                        request_id: request_id_from_headers(&headers),
                        retry_after: retry_after_from_headers(&headers),
                    });
                },
            }

            let remaining = context
                .attempt_deadline
                .saturating_duration_since(tokio::time::Instant::now());
            let body = read_error_body(response, ERROR_BODY_MAX, remaining).await;
            let retry_after = retry_after_from_headers(&headers);
            let business_probe = if body.complete {
                response_business_probe(status, prepped.response_mode, &headers, &body.bytes)
            } else {
                decode::ProbeOutcome::Clean
            };
            let business_code = body
                .complete
                .then(|| business_code_from_probe(&business_probe))
                .flatten();
            return Ok(AttemptStep::Final(TransportResponse {
                status,
                headers,
                body: body.bytes,
                business_probe,
                business_code,
                error_body_complete: body.complete,
                success_statuses: prepped.success_statuses,
                response_mode: prepped.response_mode,
                attempts: context.attempt,
                retry_after,
                in_flight_permit: None,
            }));
        }

        if (200..300).contains(&status)
            && (!prepped.success_statuses.contains(&status)
                || headers.contains_key(reqwest::header::CONTENT_RANGE))
        {
            return Ok(AttemptStep::Final(TransportResponse {
                status,
                headers,
                body: Bytes::new(),
                business_probe: decode::ProbeOutcome::Clean,
                business_code: None,
                error_body_complete: true,
                success_statuses: prepped.success_statuses,
                response_mode: prepped.response_mode,
                attempts: context.attempt,
                retry_after: retry_after_from_headers(response.headers()),
                in_flight_permit: None,
            }));
        }

        let limit = response_limit(status, prepped.response_mode);
        let remaining = context
            .attempt_deadline
            .saturating_duration_since(tokio::time::Instant::now());
        let body = if (200..300).contains(&status) {
            read_body(response, limit, remaining)
                .await
                .map(|bytes| (bytes, true))
        } else {
            let body = read_error_body(response, limit, remaining).await;
            Ok((body.bytes, body.complete))
        };
        let outcome = match body {
            Ok((body, error_body_complete)) => AttemptOutcome::Response {
                status,
                headers,
                body,
                error_body_complete,
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
        sensitive_headers: &[SensitiveHeader],
    ) -> ZaiResult<reqwest::RequestBuilder> {
        self.build_request_with_accept(method, url, body, response_mode.accept(), sensitive_headers)
            .await
    }

    async fn build_request_with_accept(
        &self,
        method: &str,
        url: &str,
        body: &request::BodyKind<'_>,
        accept: &'static str,
        sensitive_headers: &[SensitiveHeader],
    ) -> ZaiResult<reqwest::RequestBuilder> {
        let m = reqwest::Method::from_bytes(method.as_bytes())
            .map_err(|_| invalid("invalid HTTP method"))?;
        let mut auth =
            reqwest::header::HeaderValue::from_str(&format!("Bearer {}", self.secret.expose()))
                .map_err(|_| invalid("invalid authorization header"))?;
        auth.set_sensitive(true);
        let parsed_url = url::Url::parse(url).map_err(|_| invalid("invalid request URL"))?;
        let reqwest = if crate::client::endpoint::url_is_loopback(&parsed_url) {
            self.direct_reqwest
                .as_ref()
                .ok_or_else(|| invalid("loopback request requires a proxy-free transport pool"))?
        } else {
            &self.reqwest
        };
        let mut rb = reqwest
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
        for header in sensitive_headers {
            rb = rb.header(header.name().clone(), header.value().clone());
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

fn redact_sensitive_header_values(text: &str, headers: &[SensitiveHeader]) -> String {
    let mut redacted = text.to_owned();
    for header in headers {
        if let Ok(value) = header.value().to_str() {
            redacted = redacted.replace(value, "[REDACTED]");
        }
    }
    redacted
}

fn omit_sensitive_request_id(
    request_id: Option<String>,
    headers: &[SensitiveHeader],
) -> Option<String> {
    request_id.filter(|request_id| !contains_sensitive_header_value(request_id, headers))
}

fn business_code_contains_sensitive_header_value(
    code: &serde_json::Value,
    headers: &[SensitiveHeader],
) -> bool {
    contains_sensitive_header_value(&code.to_string(), headers)
        || code
            .as_str()
            .is_some_and(|code| contains_sensitive_header_value(code, headers))
}

fn contains_sensitive_header_value(text: &str, headers: &[SensitiveHeader]) -> bool {
    headers.iter().any(|header| {
        header
            .value()
            .to_str()
            .map_or(true, |secret| text.contains(secret))
    })
}

impl SseStreamState {
    async fn next_item(mut self) -> Option<(ZaiResult<Bytes>, Self)> {
        if self.terminated {
            return None;
        }

        loop {
            let Some(inner) = self.inner.as_mut() else {
                self.terminate();
                return Some((
                    Err(invalid("SSE response stream was not initialized")
                        .with_request_metadata(self.metadata.clone())),
                    self,
                ));
            };

            // Prefer an already-buffered non-empty network chunk over an
            // elapsed timer. This prevents a paused consumer from creating a
            // false idle timeout. Empty HTTP/2 DATA frames never reset the
            // deadline, and the explicit check prevents an always-ready peer
            // from starving the timer.
            let next = tokio::select! {
                biased;
                next = inner.next() => Some(next),
                () = tokio::time::sleep_until(self.idle_deadline) => None,
            };
            match next {
                Some(Some(Ok(chunk))) if chunk.is_empty() => {
                    if tokio::time::Instant::now() >= self.idle_deadline {
                        let error = timeout_sse_idle().with_request_metadata(self.metadata.clone());
                        self.terminate();
                        return Some((Err(error), self));
                    }
                    tokio::task::yield_now().await;
                },
                Some(Some(Ok(chunk))) if (chunk.len() as u64) <= JSON_RESPONSE_MAX => {
                    self.idle_deadline = tokio::time::Instant::now() + self.idle_timeout;
                    return Some((Ok(chunk), self));
                },
                Some(Some(Ok(_))) => {
                    let error = response_too_large(JSON_RESPONSE_MAX)
                        .with_request_metadata(self.metadata.clone());
                    self.terminate();
                    return Some((Err(error), self));
                },
                Some(Some(Err(error))) => {
                    let error = ZaiError::from(error).with_request_metadata(self.metadata.clone());
                    self.terminate();
                    return Some((Err(error), self));
                },
                Some(None) => {
                    self.terminate();
                    return None;
                },
                None => {
                    let error = timeout_sse_idle().with_request_metadata(self.metadata.clone());
                    self.terminate();
                    return Some((Err(error), self));
                },
            }
        }
    }

    fn terminate(&mut self) {
        self.terminated = true;
        self.inner = None;
        self.in_flight_permit = None;
    }
}

impl FileStreamState {
    async fn open_response(&mut self) -> ZaiResult<()> {
        loop {
            if tokio::time::Instant::now() >= self.overall_deadline {
                return Err(self.annotate(timeout_overall()));
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
                self.transport.build_request(
                    self.method,
                    &self.url,
                    &body,
                    ResponseMode::File,
                    &self.sensitive_headers,
                ),
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
            self.request_id = request_id_from_headers(&headers);
            self.retry_after = retry_after_from_headers(&headers);
            self.refresh_lease_errors();
            if (300..400).contains(&status) {
                let location = redirect_location(&headers).map_err(|error| self.annotate(error))?;
                let current =
                    url::Url::parse(&self.url).map_err(|_| invalid("current url parse"))?;
                match follow_redirect(
                    &current,
                    status,
                    location,
                    self.safety,
                    self.method,
                    self.redirect_hops,
                ) {
                    Ok(Some(target)) => {
                        self.redirect_hops += 1;
                        self.url = target.to_string();
                        continue;
                    },
                    Ok(None) => {},
                    Err(error) => return Err(self.annotate(error)),
                }
            }

            let content_type = headers
                .get(reqwest::header::CONTENT_TYPE)
                .and_then(|value| value.to_str().ok())
                .unwrap_or_default();
            let content_type_error = ResponseMode::File.validate_content_type(content_type).err();
            let is_json_response = decode::validate_json_content_type(content_type).is_ok();
            let has_content_range = headers.contains_key(reqwest::header::CONTENT_RANGE);
            // The complete-object file operation accepts exactly an unranged
            // 200. Enforce that invariant from headers alone, including for a
            // JSON-looking response: an invalid 2xx must not make a slow body
            // observable or let an in-body business code trigger a retry.
            if (200..300).contains(&status) && (status != 200 || has_content_range) {
                let detail = if has_content_range {
                    "file response unexpectedly included Content-Range"
                } else {
                    "file response requires HTTP 200 OK"
                };
                return Err(self.annotate(invalid_response(detail)));
            }
            if status == 200 && !has_content_range && content_type_error.is_none() {
                let announced_length = headers
                    .get(reqwest::header::CONTENT_LENGTH)
                    .and_then(|value| value.to_str().ok())
                    .and_then(|value| value.parse::<u64>().ok());
                if announced_length.is_some_and(|length| length > MULTIPART_FILE_BYTES_MAX) {
                    return Err(self.annotate(response_too_large(MULTIPART_FILE_BYTES_MAX)));
                }
                self.body = Some(Box::pin(response.bytes_stream()));
                return Ok(());
            }

            // Reject remaining non-JSON 2xx protocol violations without
            // waiting for an untrusted body. An unranged JSON 200 may still be
            // the provider's business-error envelope, so retain the bounded
            // decode/retry path below for that one case.
            if (200..300).contains(&status) && !is_json_response {
                return Err(self.annotate(content_type_error.unwrap_or_else(|| {
                    invalid_response("file response did not use application/octet-stream")
                })));
            }

            let remaining = self
                .attempt_deadline
                .saturating_duration_since(tokio::time::Instant::now());
            let response_body = read_error_body(response, ERROR_BODY_MAX, remaining).await;

            let should_probe_business_error = !(200..300).contains(&status) || is_json_response;
            let business_probe = if response_body.complete {
                probe_business_response(should_probe_business_error, &response_body.bytes)
            } else {
                decode::ProbeOutcome::Clean
            };
            if let Some(request_id) = request_id_from_probe(&business_probe) {
                self.request_id = Some(request_id);
                self.refresh_lease_errors();
            }
            let business_code = business_code_from_probe(&business_probe);

            if self.safety == RetrySafety::Idempotent
                && is_retryable_outcome(status, business_code)
                && self.attempt < self.max_attempts
            {
                self.schedule_retry(self.retry_after).await?;
                continue;
            }
            match business_probe {
                decode::ProbeOutcome::Error(error) => {
                    return Err(self.annotate(api_error(status, error)));
                },
                decode::ProbeOutcome::Ambiguous => {
                    return Err(self.annotate(ambiguous_business_error(status)));
                },
                decode::ProbeOutcome::Malformed if !(200..300).contains(&status) => {
                    return Err(self.annotate(malformed_business_error(status)));
                },
                decode::ProbeOutcome::Clean | decode::ProbeOutcome::Malformed => {},
            }
            if !response_body.complete {
                return Err(self.annotate(incomplete_error_body(status)));
            }
            // This operation always requests the complete object and never
            // sends a Range header. Accepting 206 would silently publish a
            // truncated file, while accepting 204 would turn an unexpected
            // no-content response into a successful empty download. A
            // Content-Range on 200 is equally ambiguous and must fail closed.
            if (200..300).contains(&status) {
                let detail = if has_content_range {
                    "file response unexpectedly included Content-Range"
                } else if status != 200 {
                    "file response requires HTTP 200 OK"
                } else {
                    "file response did not use application/octet-stream"
                };
                return Err(self.annotate(invalid_response(detail)));
            }
            if !(200..300).contains(&status) {
                return Err(self.annotate(ZaiError::from_api_response(
                    status,
                    business_code.unwrap_or(0),
                    String::from_utf8_lossy(&response_body.bytes).into_owned(),
                )));
            }
            return Err(self.annotate(content_type_error.unwrap_or_else(|| {
                invalid("file response did not use application/octet-stream")
            })));
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
                    self.terminate();
                    return Some((Err(error), self));
                }
                continue;
            }

            let next = match self.body.as_mut() {
                Some(body) => tokio::time::timeout(remaining, body.next()).await,
                None => {
                    self.terminate();
                    return Some((
                        Err(self.annotate(invalid("file response stream was not initialized"))),
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
                            self.terminate();
                            return Some((
                                Err(self.annotate(response_too_large(MULTIPART_FILE_BYTES_MAX))),
                                self,
                            ));
                        },
                    };
                    self.total = next_total;
                    self.delivered = true;
                    return Some((Ok(chunk), self));
                },
                Ok(Some(Err(error))) => {
                    if let Err(error) = self.retry_body_failure(error.into()).await {
                        self.terminate();
                        return Some((Err(error), self));
                    }
                },
                Ok(None) => {
                    self.terminate();
                    return None;
                },
                Err(_) => {
                    let error = self.current_timeout();
                    if let Err(error) = self.retry_body_failure(error).await {
                        self.terminate();
                        return Some((Err(error), self));
                    }
                },
            }
        }
    }

    fn terminate(&mut self) {
        self.terminated = true;
        self.body = None;
        self.in_flight_permit = None;
    }

    async fn retry_body_failure(&mut self, error: ZaiError) -> ZaiResult<()> {
        if self.delivered || self.attempt >= self.max_attempts {
            return Err(self.annotate(error));
        }
        self.body = None;
        self.schedule_retry(None).await?;
        self.open_response().await
    }

    async fn retry_transient(&mut self, error: ZaiError) -> ZaiResult<()> {
        if self.attempt >= self.max_attempts {
            return Err(self.annotate(error));
        }
        self.schedule_retry(None).await
    }

    async fn schedule_retry(&mut self, hint: Option<Duration>) -> ZaiResult<()> {
        let computed = backoff_delay(u32::from(self.attempt) - 1, self.transport.jitter.as_ref());
        let delay = reconcile_retry_after(hint, computed);
        if !delay_fits_before(delay, self.overall_deadline) {
            return Err(self.annotate(timeout_overall()));
        }
        tokio::time::sleep(delay).await;
        if tokio::time::Instant::now() >= self.overall_deadline {
            return Err(self.annotate(timeout_overall()));
        }
        self.attempt += 1;
        self.attempt_deadline =
            (tokio::time::Instant::now() + self.attempt_timeout).min(self.overall_deadline);
        self.refresh_lease_errors();
        Ok(())
    }

    fn current_timeout(&self) -> ZaiError {
        if tokio::time::Instant::now() >= self.overall_deadline {
            self.annotate(timeout_overall())
        } else {
            self.annotate(timeout_attempt())
        }
    }

    fn annotate(&self, error: ZaiError) -> ZaiError {
        annotate_request_error(
            error,
            self.attempt,
            self.request_id.clone(),
            self.retry_after,
        )
    }

    fn refresh_lease_errors(&self) {
        Self::replace_lease_error(&self.lease_expiry_error, self.annotate(timeout_overall()));
        Self::replace_lease_error(
            &self.lease_fail_closed_error,
            self.annotate(stream_lease_unavailable()),
        );
    }

    fn replace_lease_error(slot: &Mutex<ZaiError>, error: ZaiError) {
        match slot.lock() {
            Ok(mut slot) => *slot = error,
            Err(poisoned) => {
                *poisoned.into_inner() = error;
                slot.clear_poison();
            },
        }
    }
}

fn effective_max_attempts(safety: RetrySafety, configured: u8) -> u8 {
    match safety {
        RetrySafety::Idempotent => configured.max(1),
        RetrySafety::NonIdempotent => 1,
    }
}

fn effective_stream_consumer_timeout(
    global: Duration,
    scoped: Option<Duration>,
    sse_idle: Duration,
) -> Duration {
    let configured = scoped.unwrap_or(global).min(global);
    configured.max(sse_idle.saturating_add(SSE_CONSUMER_IDLE_GRACE))
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

fn sanitize_request_id(value: Option<&str>) -> Option<String> {
    let value = value?;
    if value.is_empty()
        || value.len() > 128
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
    {
        return None;
    }
    Some(value.to_owned())
}

fn request_id_from_headers(headers: &reqwest::header::HeaderMap) -> Option<String> {
    ["x-request-id", "request-id", "x-log-id"]
        .into_iter()
        .find_map(|name| {
            headers
                .get(name)
                .and_then(|value| value.to_str().ok())
                .and_then(|value| sanitize_request_id(Some(value)))
        })
}

fn retry_after_from_headers(headers: &reqwest::header::HeaderMap) -> Option<Duration> {
    headers
        .get(reqwest::header::RETRY_AFTER)
        .and_then(|value| value.to_str().ok())
        .and_then(parse_retry_after)
}

fn retry_after_from_headers_omitting_sensitive_values(
    headers: &reqwest::header::HeaderMap,
    sensitive_headers: &[SensitiveHeader],
) -> Option<Duration> {
    let value = headers.get(reqwest::header::RETRY_AFTER)?.to_str().ok()?;
    if contains_sensitive_header_value(value, sensitive_headers) {
        return None;
    }
    parse_retry_after(value)
}

fn annotate_request_error(
    error: ZaiError,
    attempts: u8,
    request_id: Option<String>,
    retry_after: Option<Duration>,
) -> ZaiError {
    error.with_request_metadata(
        RequestErrorMetadata::for_attempts(attempts)
            .with_request_id(request_id)
            .with_retry_after(retry_after),
    )
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

fn should_probe_business_error(
    status: u16,
    mode: ResponseMode,
    headers: &reqwest::header::HeaderMap,
) -> bool {
    if mode == ResponseMode::Json || !(200..300).contains(&status) {
        return true;
    }
    headers
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|content_type| decode::validate_json_content_type(content_type).is_ok())
}

fn response_business_probe(
    status: u16,
    mode: ResponseMode,
    headers: &reqwest::header::HeaderMap,
    body: &[u8],
) -> decode::ProbeOutcome {
    probe_business_response(should_probe_business_error(status, mode, headers), body)
}

fn probe_business_response(should_probe: bool, body: &[u8]) -> decode::ProbeOutcome {
    if !should_probe {
        return decode::ProbeOutcome::Clean;
    }
    std::str::from_utf8(body)
        .map(decode::probe_error_envelope)
        .unwrap_or(decode::ProbeOutcome::Malformed)
}

fn request_id_from_probe(probe: &decode::ProbeOutcome) -> Option<String> {
    match probe {
        decode::ProbeOutcome::Error(error) => sanitize_request_id(error.request_id.as_deref()),
        decode::ProbeOutcome::Clean
        | decode::ProbeOutcome::Ambiguous
        | decode::ProbeOutcome::Malformed => None,
    }
}

fn business_code_from_probe(probe: &decode::ProbeOutcome) -> Option<u16> {
    match probe {
        decode::ProbeOutcome::Error(error) => error.code.as_ref().and_then(parse_business_code),
        decode::ProbeOutcome::Clean
        | decode::ProbeOutcome::Ambiguous
        | decode::ProbeOutcome::Malformed => None,
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

/// Extract a scalar business code only from a complete, unambiguous JSON
/// business-error envelope.
#[cfg(feature = "realtime")]
pub(crate) fn business_code_from_complete_json(body: &[u8]) -> Option<u16> {
    let body = std::str::from_utf8(body).ok()?;
    let decode::ProbeOutcome::Error(error) = decode::probe_error_envelope(body) else {
        return None;
    };
    error.code.as_ref().and_then(parse_business_code)
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

struct BoundedErrorBody {
    bytes: Bytes,
    /// True only when EOF was observed before a byte cap, stream error, or
    /// deadline. A capped prefix is diagnostic data, not a trustworthy JSON
    /// document.
    complete: bool,
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

/// Read at most `limit` diagnostic bytes from an error response.
///
/// Once response headers are available, their HTTP status is more actionable
/// than an oversized, stalled, or disconnected error page. This reader stops
/// at the byte/deadline boundary and returns the prefix accumulated so far;
/// successful response bodies continue to use [`read_body`] and fail closed on
/// truncation.
async fn read_error_body(
    resp: reqwest::Response,
    limit: u64,
    timeout: Duration,
) -> BoundedErrorBody {
    read_error_stream(resp.bytes_stream(), limit, timeout).await
}

async fn read_error_stream<S, E>(stream: S, limit: u64, timeout: Duration) -> BoundedErrorBody
where
    S: Stream<Item = Result<Bytes, E>>,
{
    if timeout.is_zero() || limit == 0 {
        return BoundedErrorBody {
            bytes: Bytes::new(),
            complete: false,
        };
    }
    let initial_capacity = usize::try_from(limit.min(64 * 1024)).unwrap_or(64 * 1024);
    let mut body = BytesMut::with_capacity(initial_capacity);
    let mut stream = Box::pin(stream);
    let deadline = tokio::time::Instant::now() + timeout;

    loop {
        if tokio::time::Instant::now() >= deadline {
            return BoundedErrorBody {
                bytes: body.freeze(),
                complete: false,
            };
        }
        let next = tokio::select! {
            biased;
            () = tokio::time::sleep_until(deadline) => {
                return BoundedErrorBody {
                    bytes: body.freeze(),
                    complete: false,
                };
            },
            chunk = stream.next() => chunk,
        };
        let Some(next) = next else {
            return BoundedErrorBody {
                bytes: body.freeze(),
                complete: true,
            };
        };
        let Ok(chunk) = next else {
            return BoundedErrorBody {
                bytes: body.freeze(),
                complete: false,
            };
        };
        // Empty HTTP/2 DATA frames carry no diagnostic bytes. Ignoring them,
        // together with the deadline-first biased select above, prevents an
        // always-ready peer from starving an already-expired timer.
        if chunk.is_empty() {
            tokio::task::yield_now().await;
            continue;
        }
        let remaining = limit.saturating_sub(body.len() as u64);
        if remaining == 0 {
            return BoundedErrorBody {
                bytes: body.freeze(),
                complete: false,
            };
        }
        let keep = usize::try_from(remaining.min(chunk.len() as u64)).unwrap_or(chunk.len());
        body.extend_from_slice(&chunk[..keep]);
        if keep < chunk.len() || body.len() as u64 == limit {
            return BoundedErrorBody {
                bytes: body.freeze(),
                complete: false,
            };
        }
    }
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

fn invalid_response(message: &str) -> ZaiError {
    ZaiError::ApiError {
        code: codes::SDK_VALIDATION,
        message: message.to_string(),
    }
}

fn redirect_location(headers: &reqwest::header::HeaderMap) -> ZaiResult<&str> {
    match headers.get(reqwest::header::LOCATION) {
        None => Ok(""),
        Some(value) => value
            .to_str()
            .map_err(|_| invalid_response("redirect: Location header was not valid text")),
    }
}

const AMBIGUOUS_BUSINESS_ERROR_MESSAGE: &str =
    "ambiguous JSON business-error envelope (duplicate reserved field)";
const MALFORMED_BUSINESS_ERROR_MESSAGE: &str = "malformed JSON business-error diagnostic";
const INCOMPLETE_ERROR_BODY_MESSAGE: &str = "HTTP error response body was unavailable or truncated";

fn ambiguous_business_error(status: u16) -> ZaiError {
    if (200..300).contains(&status) {
        invalid_response(AMBIGUOUS_BUSINESS_ERROR_MESSAGE)
    } else {
        ZaiError::from_api_response(status, 0, AMBIGUOUS_BUSINESS_ERROR_MESSAGE.to_owned())
    }
}

fn malformed_business_error(status: u16) -> ZaiError {
    if (200..300).contains(&status) {
        invalid_response(MALFORMED_BUSINESS_ERROR_MESSAGE)
    } else {
        ZaiError::from_api_response(status, 0, MALFORMED_BUSINESS_ERROR_MESSAGE.to_owned())
    }
}

fn incomplete_error_body(status: u16) -> ZaiError {
    if (200..300).contains(&status) {
        invalid_response(INCOMPLETE_ERROR_BODY_MESSAGE)
    } else {
        ZaiError::from_api_response(status, 0, INCOMPLETE_ERROR_BODY_MESSAGE.to_owned())
    }
}

fn timeout_queue() -> ZaiError {
    ZaiError::ApiError {
        code: codes::SDK_TIMEOUT,
        message: "HTTP concurrency queue timeout exceeded".to_string(),
    }
    .with_request_metadata(
        RequestErrorMetadata::for_attempts(0).with_timeout_phase(TimeoutPhase::Queue),
    )
}

fn concurrency_limiter_closed() -> ZaiError {
    ZaiError::ApiError {
        code: codes::SDK_IO,
        message: "HTTP concurrency limiter is unavailable".to_string(),
    }
    .with_request_metadata(RequestErrorMetadata::for_attempts(0))
}

fn timeout_attempt() -> ZaiError {
    ZaiError::ApiError {
        code: codes::SDK_TIMEOUT,
        message: "per-attempt timeout exceeded".to_string(),
    }
    .with_request_metadata(
        RequestErrorMetadata::default().with_timeout_phase(TimeoutPhase::Attempt),
    )
}

fn timeout_overall() -> ZaiError {
    ZaiError::ApiError {
        code: codes::SDK_TIMEOUT,
        message: "overall deadline exceeded".to_string(),
    }
    .with_request_metadata(
        RequestErrorMetadata::default().with_timeout_phase(TimeoutPhase::Overall),
    )
}

fn timeout_sse_handshake() -> ZaiError {
    ZaiError::ApiError {
        code: codes::SDK_TIMEOUT,
        message: "SSE handshake timeout exceeded".to_string(),
    }
    .with_request_metadata(
        RequestErrorMetadata::default().with_timeout_phase(TimeoutPhase::SseHandshake),
    )
}

fn timeout_sse_idle() -> ZaiError {
    ZaiError::ApiError {
        code: codes::SDK_TIMEOUT,
        message: "SSE idle timeout exceeded".to_string(),
    }
    .with_request_metadata(
        RequestErrorMetadata::default().with_timeout_phase(TimeoutPhase::SseIdle),
    )
}

fn timeout_stream_consumer() -> ZaiError {
    ZaiError::ApiError {
        code: codes::SDK_TIMEOUT,
        message: "raw response stream consumer timeout exceeded".to_string(),
    }
    .with_request_metadata(
        RequestErrorMetadata::default().with_timeout_phase(TimeoutPhase::StreamConsumer),
    )
}

fn stream_lease_unavailable() -> ZaiError {
    ZaiError::ApiError {
        code: codes::SDK_IO,
        message: "raw response stream lease watchdog became unavailable".to_string(),
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
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

    use futures_util::task::noop_waker;

    use super::*;

    struct OneChunkThenPending {
        chunk: Option<Bytes>,
        dropped: Arc<AtomicBool>,
    }

    impl Stream for OneChunkThenPending {
        type Item = ZaiResult<Bytes>;

        fn poll_next(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            match self.chunk.take() {
                Some(chunk) => Poll::Ready(Some(Ok(chunk))),
                None => Poll::Pending,
            }
        }
    }

    impl Drop for OneChunkThenPending {
        fn drop(&mut self) {
            self.dropped.store(true, Ordering::SeqCst);
        }
    }

    struct PendingDropStream {
        dropped: Arc<AtomicBool>,
    }

    impl Stream for PendingDropStream {
        type Item = ZaiResult<Bytes>;

        fn poll_next(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            Poll::Pending
        }
    }

    impl Drop for PendingDropStream {
        fn drop(&mut self) {
            self.dropped.store(true, Ordering::SeqCst);
        }
    }

    struct ReadyCountingStream {
        polls: Arc<AtomicUsize>,
        dropped: Arc<AtomicBool>,
    }

    impl Stream for ReadyCountingStream {
        type Item = ZaiResult<Bytes>;

        fn poll_next(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            self.polls.fetch_add(1, Ordering::SeqCst);
            Poll::Ready(Some(Ok(Bytes::from_static(b"too late"))))
        }
    }

    impl Drop for ReadyCountingStream {
        fn drop(&mut self) {
            self.dropped.store(true, Ordering::SeqCst);
        }
    }

    struct PanicOnDropStream;

    impl Stream for PanicOnDropStream {
        type Item = ZaiResult<Bytes>;

        fn poll_next(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            Poll::Pending
        }
    }

    impl Drop for PanicOnDropStream {
        fn drop(&mut self) {
            panic!("mock inner stream drop panic");
        }
    }

    struct DropFlag(Arc<AtomicBool>);

    impl Drop for DropFlag {
        fn drop(&mut self) {
            self.0.store(true, Ordering::SeqCst);
        }
    }

    struct PermitOrderProbe {
        body_dropped: Arc<AtomicBool>,
        order_preserved: Arc<AtomicBool>,
    }

    impl Drop for PermitOrderProbe {
        fn drop(&mut self) {
            self.order_preserved
                .store(self.body_dropped.load(Ordering::SeqCst), Ordering::SeqCst);
        }
    }

    struct PermitHoldingStream {
        // Field order intentionally mirrors FileStreamState/SseStreamState:
        // response-owned data is dropped before admission ownership.
        _body: DropFlag,
        _permit_order: PermitOrderProbe,
        _permit: OwnedSemaphorePermit,
    }

    impl Stream for PermitHoldingStream {
        type Item = ZaiResult<Bytes>;

        fn poll_next(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            Poll::Pending
        }
    }

    fn boxed_pending(dropped: Arc<AtomicBool>) -> SseByteStream {
        Box::pin(PendingDropStream { dropped })
    }

    fn test_consumer_timeout() -> ZaiError {
        timeout_stream_consumer().with_request_metadata(RequestErrorMetadata::for_attempts(1))
    }

    fn test_fail_closed_error() -> WholeStreamErrorSource {
        WholeStreamErrorSource::Fixed(
            stream_lease_unavailable().with_request_metadata(RequestErrorMetadata::for_attempts(1)),
        )
    }

    fn assert_timeout_phase(error: &ZaiError, phase: TimeoutPhase) {
        assert_eq!(
            error
                .request_metadata()
                .and_then(RequestErrorMetadata::timeout_phase),
            Some(phase)
        );
    }

    #[test]
    fn timeout_defaults_match_plan() {
        let t = TimeoutPolicy::default();
        assert_eq!(t.attempt, Duration::from_secs(60));
        assert_eq!(t.overall, Duration::from_secs(120));
    }

    #[test]
    fn sse_consumer_timeout_is_scoped_but_never_races_idle() {
        assert_eq!(
            effective_stream_consumer_timeout(
                Duration::from_secs(10),
                Some(Duration::from_secs(20)),
                Duration::from_secs(1),
            ),
            Duration::from_secs(10),
            "a scoped value cannot raise the configured base"
        );
        assert_eq!(
            effective_stream_consumer_timeout(
                Duration::from_secs(10),
                Some(Duration::from_secs(5)),
                Duration::from_secs(1),
            ),
            Duration::from_secs(5),
            "a safe scoped value lowers the configured base"
        );
        assert_eq!(
            effective_stream_consumer_timeout(
                Duration::from_millis(50),
                None,
                Duration::from_millis(1),
            ),
            Duration::from_millis(1001),
            "the idle deadline keeps deterministic priority over reclamation"
        );
        assert_eq!(
            effective_stream_consumer_timeout(
                crate::client::HttpConcurrencyConfig::MAX_STREAM_CONSUMER_TIMEOUT,
                None,
                crate::client::HttpTransportConfig::MAX_REQUEST_TIMEOUT,
            ),
            crate::client::HttpTransportConfig::MAX_REQUEST_TIMEOUT + SSE_CONSUMER_IDLE_GRACE,
            "the derived safety floor is one second beyond the configured cap"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn whole_stream_sliding_deadline_resets_on_raw_chunk_and_errors_once() {
        let dropped = Arc::new(AtomicBool::new(false));
        let timeout = Duration::from_secs(5);
        let inner: SseByteStream = Box::pin(OneChunkThenPending {
            chunk: Some(Bytes::from_static(b"chunk")),
            dropped: Arc::clone(&dropped),
        });
        let mut lease = WholeStreamLease::new(
            inner,
            tokio::time::Instant::now() + timeout,
            Some(timeout),
            WholeStreamErrorSource::Fixed(test_consumer_timeout()),
            test_fail_closed_error(),
        )
        .unwrap();

        tokio::time::advance(Duration::from_secs(4)).await;
        assert_eq!(
            lease.next().await.unwrap().unwrap(),
            Bytes::from_static(b"chunk")
        );
        assert!(
            lease.shared.consumer_waker.take().is_none(),
            "a ready item must not retain the caller task's waker"
        );

        // Cross the original deadline; successful raw progress moved it by a
        // full interval and the watchdog must honor the new generation.
        tokio::time::advance(Duration::from_secs(4)).await;
        tokio::task::yield_now().await;
        assert!(!dropped.load(Ordering::SeqCst));

        tokio::time::advance(Duration::from_secs(1)).await;
        tokio::task::yield_now().await;
        let error = lease.next().await.unwrap().unwrap_err();
        assert_timeout_phase(&error, TimeoutPhase::StreamConsumer);
        assert!(lease.shared.consumer_waker.take().is_none());
        assert!(dropped.load(Ordering::SeqCst));
        assert!(lease.next().await.is_none());
    }

    #[tokio::test(start_paused = true)]
    async fn whole_stream_fixed_deadline_never_resets_on_progress() {
        let dropped = Arc::new(AtomicBool::new(false));
        let inner: FileByteStream = Box::pin(OneChunkThenPending {
            chunk: Some(Bytes::from_static(b"chunk")),
            dropped: Arc::clone(&dropped),
        });
        let mut lease = WholeStreamLease::new(
            inner,
            tokio::time::Instant::now() + Duration::from_secs(5),
            None,
            WholeStreamErrorSource::Fixed(
                timeout_overall().with_request_metadata(RequestErrorMetadata::for_attempts(2)),
            ),
            test_fail_closed_error(),
        )
        .unwrap();

        tokio::time::advance(Duration::from_secs(4)).await;
        assert!(lease.next().await.unwrap().is_ok());
        tokio::time::advance(Duration::from_secs(1)).await;
        tokio::task::yield_now().await;

        let error = lease.next().await.unwrap().unwrap_err();
        assert_timeout_phase(&error, TimeoutPhase::Overall);
        assert_eq!(error.request_metadata().unwrap().attempts(), 2);
        assert!(dropped.load(Ordering::SeqCst));
        assert!(lease.next().await.is_none());
    }

    #[tokio::test(start_paused = true)]
    async fn expired_lease_never_polls_or_yields_an_already_ready_chunk() {
        let polls = Arc::new(AtomicUsize::new(0));
        let dropped = Arc::new(AtomicBool::new(false));
        let inner: SseByteStream = Box::pin(ReadyCountingStream {
            polls: Arc::clone(&polls),
            dropped: Arc::clone(&dropped),
        });
        let mut lease = WholeStreamLease::new(
            inner,
            tokio::time::Instant::now(),
            Some(Duration::from_secs(5)),
            WholeStreamErrorSource::Fixed(test_consumer_timeout()),
            test_fail_closed_error(),
        )
        .unwrap();

        let error = lease.next().await.unwrap().unwrap_err();
        assert_timeout_phase(&error, TimeoutPhase::StreamConsumer);
        assert_eq!(
            polls.load(Ordering::SeqCst),
            0,
            "watchdog must not prefetch"
        );
        assert!(dropped.load(Ordering::SeqCst));
        assert!(lease.next().await.is_none());
    }

    #[tokio::test]
    async fn whole_stream_drop_synchronously_closes_inner() {
        let dropped = Arc::new(AtomicBool::new(false));
        let lease = WholeStreamLease::new(
            boxed_pending(Arc::clone(&dropped)),
            tokio::time::Instant::now() + Duration::from_secs(60),
            Some(Duration::from_secs(60)),
            WholeStreamErrorSource::Fixed(test_consumer_timeout()),
            test_fail_closed_error(),
        )
        .unwrap();

        drop(lease);
        assert!(dropped.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn watchdog_abort_drops_body_before_releasing_permit() {
        let body_dropped = Arc::new(AtomicBool::new(false));
        let order_preserved = Arc::new(AtomicBool::new(false));
        let semaphore = Arc::new(Semaphore::new(1));
        let permit = Arc::clone(&semaphore).acquire_owned().await.unwrap();
        let inner: SseByteStream = Box::pin(PermitHoldingStream {
            _body: DropFlag(Arc::clone(&body_dropped)),
            _permit_order: PermitOrderProbe {
                body_dropped: Arc::clone(&body_dropped),
                order_preserved: Arc::clone(&order_preserved),
            },
            _permit: permit,
        });
        let mut lease = WholeStreamLease::new(
            inner,
            tokio::time::Instant::now() + Duration::from_secs(60),
            Some(Duration::from_secs(60)),
            WholeStreamErrorSource::Fixed(test_consumer_timeout()),
            test_fail_closed_error(),
        )
        .unwrap();

        lease.watchdog.abort();
        for _ in 0..16 {
            if body_dropped.load(Ordering::SeqCst) {
                break;
            }
            tokio::task::yield_now().await;
        }

        assert!(body_dropped.load(Ordering::SeqCst));
        assert!(order_preserved.load(Ordering::SeqCst));
        let reacquired = semaphore.try_acquire_owned().unwrap();
        let error = lease.next().await.unwrap().unwrap_err();
        assert_eq!(error.code(), Some(codes::SDK_IO));
        assert!(lease.next().await.is_none());
        drop(reacquired);
    }

    #[test]
    fn runtime_shutdown_drops_watchdog_guard_and_inner() {
        let dropped = Arc::new(AtomicBool::new(false));
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .unwrap();
        let lease = runtime.block_on(async {
            WholeStreamLease::new(
                boxed_pending(Arc::clone(&dropped)),
                tokio::time::Instant::now() + Duration::from_secs(60),
                Some(Duration::from_secs(60)),
                WholeStreamErrorSource::Fixed(test_consumer_timeout()),
                test_fail_closed_error(),
            )
            .unwrap()
        });

        drop(runtime);
        assert!(dropped.load(Ordering::SeqCst));
        drop(lease);
    }

    #[test]
    fn constructor_without_runtime_fails_closed_without_panicking() {
        let dropped = Arc::new(AtomicBool::new(false));
        let result = WholeStreamLease::new(
            boxed_pending(Arc::clone(&dropped)),
            tokio::time::Instant::now() + Duration::from_secs(60),
            Some(Duration::from_secs(60)),
            WholeStreamErrorSource::Fixed(test_consumer_timeout()),
            test_fail_closed_error(),
        );

        assert!(result.is_err());
        assert!(dropped.load(Ordering::SeqCst));
    }

    #[test]
    fn poisoned_slot_drops_inner_and_yields_one_safe_error() {
        let dropped = Arc::new(AtomicBool::new(false));
        let shared = WholeStreamLeaseShared::new(
            boxed_pending(Arc::clone(&dropped)),
            tokio::time::Instant::now() + Duration::from_secs(60),
            Some(Duration::from_secs(60)),
            WholeStreamErrorSource::Fixed(test_consumer_timeout()),
            test_fail_closed_error(),
        );
        let poison_target = Arc::clone(&shared);
        assert!(
            std::thread::spawn(move || {
                let _guard = poison_target.state.lock().unwrap();
                panic!("poison stream lease slot");
            })
            .join()
            .is_err()
        );

        let waker = noop_waker();
        let mut cx = Context::from_waker(&waker);
        let error = match shared.poll_inner(&mut cx) {
            Poll::Ready(Some(Err(error))) => error,
            other => panic!("expected one fail-closed error, got {other:?}"),
        };
        assert_eq!(error.code(), Some(codes::SDK_IO));
        assert!(dropped.load(Ordering::SeqCst));
        assert!(matches!(shared.poll_inner(&mut cx), Poll::Ready(None)));
    }

    #[test]
    fn drop_panic_cannot_erase_the_precommitted_terminal_error() {
        let shared = WholeStreamLeaseShared::new(
            Box::pin(PanicOnDropStream),
            tokio::time::Instant::now(),
            Some(Duration::from_secs(60)),
            WholeStreamErrorSource::Fixed(test_consumer_timeout()),
            test_fail_closed_error(),
        );
        let waker = noop_waker();
        let mut cx = Context::from_waker(&waker);

        let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = shared.poll_inner(&mut cx);
        }));
        assert!(panic.is_err());

        let error = match shared.poll_inner(&mut cx) {
            Poll::Ready(Some(Err(error))) => error,
            other => panic!("expected the precommitted expiry error, got {other:?}"),
        };
        assert_timeout_phase(&error, TimeoutPhase::StreamConsumer);
        assert!(matches!(shared.poll_inner(&mut cx), Poll::Ready(None)));
    }

    #[test]
    fn shared_fail_closed_error_uses_latest_request_metadata() {
        let dropped = Arc::new(AtomicBool::new(false));
        let fail_closed = Arc::new(Mutex::new(
            stream_lease_unavailable().with_request_metadata(RequestErrorMetadata::for_attempts(1)),
        ));
        let shared = WholeStreamLeaseShared::new(
            boxed_pending(Arc::clone(&dropped)),
            tokio::time::Instant::now() + Duration::from_secs(60),
            None,
            WholeStreamErrorSource::Fixed(
                timeout_overall().with_request_metadata(RequestErrorMetadata::for_attempts(1)),
            ),
            WholeStreamErrorSource::Shared(Arc::clone(&fail_closed)),
        );
        *fail_closed.lock().unwrap() = stream_lease_unavailable().with_request_metadata(
            RequestErrorMetadata::for_attempts(2)
                .with_request_id(Some("retry-request".to_string())),
        );

        assert!(shared.fail_closed_if_open());
        assert!(dropped.load(Ordering::SeqCst));
        let waker = noop_waker();
        let mut cx = Context::from_waker(&waker);
        let error = match shared.poll_inner(&mut cx) {
            Poll::Ready(Some(Err(error))) => error,
            other => panic!("expected refreshed fail-closed error, got {other:?}"),
        };
        let metadata = error.request_metadata().unwrap();
        assert_eq!(metadata.attempts(), 2);
        assert_eq!(metadata.request_id(), Some("retry-request"));
        assert_eq!(error.code(), Some(codes::SDK_IO));
        assert!(matches!(shared.poll_inner(&mut cx), Poll::Ready(None)));
    }

    #[test]
    fn nonidempotent_max_attempts_is_one() {
        assert_eq!(effective_max_attempts(RetrySafety::NonIdempotent, 3), 1);
        assert_eq!(effective_max_attempts(RetrySafety::Idempotent, 3), 3);
        assert_eq!(effective_max_attempts(RetrySafety::Idempotent, 1), 1);
    }

    #[test]
    fn business_error_probe_is_scoped_to_json_or_http_error_responses() {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::CONTENT_TYPE,
            reqwest::header::HeaderValue::from_static("audio/wav"),
        );
        assert!(should_probe_business_error(
            200,
            ResponseMode::Json,
            &headers
        ));
        assert!(!should_probe_business_error(
            200,
            ResponseMode::Audio,
            &headers
        ));
        assert!(should_probe_business_error(
            503,
            ResponseMode::Audio,
            &headers
        ));

        headers.insert(
            reqwest::header::CONTENT_TYPE,
            reqwest::header::HeaderValue::from_static("application/problem+json"),
        );
        assert!(should_probe_business_error(
            200,
            ResponseMode::Audio,
            &headers
        ));
    }

    #[test]
    fn transport_response_consumes_cached_probe_without_reparsing_body() {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::CONTENT_TYPE,
            reqwest::header::HeaderValue::from_static("application/json"),
        );
        let response = TransportResponse {
            status: 200,
            headers,
            body: Bytes::from_static(br#"{"code":200,"message":"body is success"}"#),
            business_probe: decode::ProbeOutcome::Error(decode::BusinessError {
                code: Some(serde_json::json!(1302)),
                message: "cached error".to_owned(),
                request_id: None,
            }),
            business_code: Some(1302),
            error_body_complete: true,
            success_statuses: &[200],
            response_mode: ResponseMode::Json,
            attempts: 1,
            retry_after: None,
            in_flight_permit: None,
        };

        let error = response.json::<serde_json::Value>().unwrap_err();
        assert_eq!(error.code(), Some(1302));
        assert_eq!(error.message(), "cached error");
    }

    #[test]
    fn cached_ambiguous_probe_blocks_typed_success_without_body_leakage() {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::CONTENT_TYPE,
            reqwest::header::HeaderValue::from_static("application/json"),
        );
        let response = TransportResponse {
            status: 200,
            headers,
            body: Bytes::from_static(br#"{"id":"private-success-payload","code":1302,"code":200}"#),
            business_probe: decode::ProbeOutcome::Ambiguous,
            business_code: None,
            error_body_complete: true,
            success_statuses: &[200],
            response_mode: ResponseMode::Json,
            attempts: 1,
            retry_after: None,
            in_flight_permit: None,
        };

        let error = response.json::<serde_json::Value>().unwrap_err();
        assert_eq!(error.code(), Some(codes::SDK_VALIDATION));
        assert_eq!(error.message(), AMBIGUOUS_BUSINESS_ERROR_MESSAGE);
        for rendered in [error.to_string(), format!("{error:?}"), error.compact()] {
            assert!(!rendered.contains("private-success-payload"));
            assert!(!rendered.contains("1302"));
        }
    }

    #[test]
    fn transport_response_uses_cached_fallback_code_without_rescanning() {
        let response = TransportResponse {
            status: 400,
            headers: reqwest::header::HeaderMap::new(),
            body: Bytes::from_static(br#"{"code":1302,"message":"body-code"}"#),
            business_probe: decode::ProbeOutcome::Clean,
            business_code: Some(1113),
            error_body_complete: true,
            success_statuses: &[200],
            response_mode: ResponseMode::Json,
            attempts: 1,
            retry_after: None,
            in_flight_permit: None,
        };

        let error = response.bytes().unwrap_err();
        assert_eq!(error.code(), Some(1113));
        assert_ne!(error.code(), Some(1302));
    }

    #[test]
    fn ambiguous_probe_uses_http_status_only() {
        assert_eq!(
            business_code_from_probe(&decode::ProbeOutcome::Ambiguous),
            None
        );

        for status in [401, 429, 503] {
            let error = ambiguous_business_error(status);
            assert_eq!(error.message(), AMBIGUOUS_BUSINESS_ERROR_MESSAGE);
            assert_eq!(error.code(), Some(status));
            assert!(!error.message().contains("private-body"));
            match status {
                401 => assert!(error.is_auth_error()),
                429 => assert!(error.is_rate_limit()),
                503 => assert!(error.is_server_error()),
                _ => unreachable!(),
            }
        }
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

    #[test]
    fn non_utf8_redirect_location_is_a_static_policy_error() {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::LOCATION,
            reqwest::header::HeaderValue::from_bytes(b"/private-\xff-location").unwrap(),
        );

        let error = redirect_location(&headers).unwrap_err();
        assert_eq!(error.code(), Some(codes::SDK_VALIDATION));
        assert_eq!(
            error.message(),
            "redirect: Location header was not valid text"
        );
        for rendered in [error.to_string(), format!("{error:?}"), error.compact()] {
            assert!(!rendered.contains("private"));
            assert!(!rendered.contains('�'));
        }
    }

    #[tokio::test]
    async fn always_ready_empty_error_frames_cannot_starve_deadline() {
        let frames =
            futures_util::stream::repeat(Ok::<Bytes, std::convert::Infallible>(Bytes::new()));
        let body = tokio::time::timeout(
            Duration::from_millis(250),
            read_error_stream(frames, ERROR_BODY_MAX, Duration::from_millis(5)),
        )
        .await
        .expect("empty frames must not starve the diagnostic deadline");
        assert!(body.bytes.is_empty());
        assert!(!body.complete);
    }

    #[test]
    fn request_id_sanitization_is_bounded_and_conservative() {
        assert_eq!(
            sanitize_request_id(Some("request_42:attempt-2")),
            Some("request_42:attempt-2".to_string())
        );
        for rejected in [
            "",
            " request-42",
            "request/42",
            "request\n42",
            &"x".repeat(129),
        ] {
            assert_eq!(sanitize_request_id(Some(rejected)), None);
        }
    }
}
