//! Record-only Realtime soak evidence for the scheduled benchmark workflow.
//!
//! The ordinary test matrix compiles this target but does not run it. The
//! weekly workflow supplies a longer duration and preserves the single JSON
//! record as an artifact. Latencies and RSS are deliberately observations,
//! not shared-runner thresholds; exact protocol, capacity, ordering, failure,
//! and liveness invariants remain hard assertions.
#![cfg(feature = "realtime")]

use std::{
    future::pending,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::Duration,
};

use async_trait::async_trait;
use bytes::Bytes;
use futures_util::{SinkExt as _, StreamExt as _};
use serde_json::{Value, json};
use tokio::{
    net::TcpListener,
    sync::{oneshot, watch},
    task::JoinHandle,
    time::{Instant, MissedTickBehavior},
};
use tokio_tungstenite::tungstenite::Message;
use zai_rs::{
    ZaiError, ZaiResult,
    client::EndpointConfig,
    model::GLM_realtime_flash,
    realtime::{
        InputAudioFormat, RealtimeClient, RealtimeTransport, RealtimeTransportConfig, WsMessage,
    },
};

const TEST_KEY: &str = "test.12345678901234567890";
const SOAK_SECONDS_ENV: &str = "ZAI_REALTIME_SOAK_SECONDS";
const DEFAULT_SOAK_SECONDS: u64 = 5;
const MAX_SOAK_SECONDS: u64 = 15 * 60;
const FRAME_PERIOD: Duration = Duration::from_millis(20);
const PCM_BYTES_PER_20_MS: usize = 16_000 * 2 / 50;
const MAX_QUEUE_CAPACITY: usize = RealtimeTransportConfig::MAX_QUEUE_CAPACITY;
const RAW_MEDIA_LIMIT: usize = 4 * 1024 * 1024;
const LIVENESS_WATCHDOG: Duration = Duration::from_secs(30);
const FEEDBACK_PONG_SAMPLES: usize = 64;
const FEEDBACK_STARVATION_LIMIT: usize = 4_096;
const FEEDBACK_AUDIO_FRAMES: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PeerPolicy {
    stopped: bool,
    delay: Duration,
}

impl PeerPolicy {
    const STOPPED: Self = Self {
        stopped: true,
        delay: Duration::ZERO,
    };
    const FAST: Self = Self {
        stopped: false,
        delay: Duration::ZERO,
    };
    const SLOWER_THAN_AUDIO: Self = Self {
        stopped: false,
        delay: Duration::from_millis(21),
    };
}

#[derive(Debug, Clone)]
struct FrameSummary {
    kind: &'static str,
    wire_bytes: usize,
}

#[derive(Debug, Default)]
struct GatedState {
    send_started: AtomicUsize,
    send_failed: AtomicUsize,
    close_count: AtomicUsize,
    fail_next: AtomicBool,
    observed: Mutex<Vec<FrameSummary>>,
}

#[derive(Clone)]
struct GatedControl {
    policy_tx: watch::Sender<PeerPolicy>,
    state: Arc<GatedState>,
}

impl GatedControl {
    fn set_policy(&self, policy: PeerPolicy) {
        self.policy_tx.send_replace(policy);
    }

    fn fail_next_send(&self) {
        self.state.fail_next.store(true, Ordering::SeqCst);
    }

    fn observed(&self) -> Vec<FrameSummary> {
        self.state
            .observed
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    fn observed_application_frames(&self) -> usize {
        self.observed()
            .into_iter()
            .filter(|frame| frame.kind != "session.update")
            .count()
    }
}

struct GatedTransport {
    policy_rx: watch::Receiver<PeerPolicy>,
    state: Arc<GatedState>,
}

fn gated_transport() -> (GatedControl, GatedTransport) {
    let (policy_tx, policy_rx) = watch::channel(PeerPolicy::STOPPED);
    let state = Arc::new(GatedState::default());
    (
        GatedControl {
            policy_tx,
            state: Arc::clone(&state),
        },
        GatedTransport { policy_rx, state },
    )
}

#[async_trait]
impl RealtimeTransport for GatedTransport {
    async fn send(&mut self, msg: String) -> ZaiResult<()> {
        self.state.send_started.fetch_add(1, Ordering::SeqCst);

        loop {
            let policy = *self.policy_rx.borrow_and_update();
            if !policy.stopped {
                if !policy.delay.is_zero() {
                    tokio::time::sleep(policy.delay).await;
                }
                break;
            }
            self.policy_rx
                .changed()
                .await
                .map_err(|_| ZaiError::ApiError {
                    code: 9_901,
                    message: "soak peer policy channel closed".to_owned(),
                })?;
        }

        if self.state.fail_next.swap(false, Ordering::SeqCst) {
            self.state.send_failed.fetch_add(1, Ordering::SeqCst);
            return Err(ZaiError::ApiError {
                code: 9_902,
                message: "synthetic soak writer failure".to_owned(),
            });
        }

        record_frame(&self.state, &msg);
        Ok(())
    }

    async fn send_confirmed(&mut self, msg: String) -> ZaiResult<()> {
        record_frame(&self.state, &msg);
        Ok(())
    }

    async fn recv(&mut self) -> ZaiResult<Option<WsMessage>> {
        pending().await
    }

    async fn close(&mut self) -> ZaiResult<()> {
        self.state.close_count.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

fn record_frame(state: &GatedState, frame: &str) {
    state
        .observed
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .push(FrameSummary {
            kind: frame_kind(frame),
            wire_bytes: frame.len(),
        });
}

fn frame_kind(frame: &str) -> &'static str {
    const TYPES: &[(&str, &str)] = &[
        (r#""type":"session.update""#, "session.update"),
        (
            r#""type":"input_audio_buffer.append""#,
            "input_audio_buffer.append",
        ),
        (
            r#""type":"input_audio_buffer.commit""#,
            "input_audio_buffer.commit",
        ),
        (r#""type":"response.create""#, "response.create"),
        (r#""type":"response.cancel""#, "response.cancel"),
    ];
    TYPES
        .iter()
        .find_map(|(pattern, kind)| frame.contains(pattern).then_some(*kind))
        .unwrap_or("unknown")
}

fn transport_config() -> RealtimeTransportConfig {
    RealtimeTransportConfig::builder()
        // Keep the record-only profile's deliberately quiet injected peer
        // alive for the largest accepted duration. Idle-heartbeat behavior is
        // covered separately; a 300-second soak must not trip the default
        // 90-second application-idle policy before its scheduled endpoint.
        .inbound_idle_timeout(Duration::from_secs(MAX_SOAK_SECONDS + 60))
        .outbound_queue_timeout(Duration::ZERO)
        .outbound_queue_capacity(MAX_QUEUE_CAPACITY)
        .writer_queue_capacity(MAX_QUEUE_CAPACITY)
        .try_build()
        .expect("soak transport policy must remain valid")
}

async fn injected_session(transport: GatedTransport) -> zai_rs::realtime::RealtimeSession {
    RealtimeClient::new(TEST_KEY)
        .session(GLM_realtime_flash {})
        .input_audio_format(InputAudioFormat::Pcm16)
        .with_transport_config(transport_config())
        .build_with_transport(transport)
        .await
        .expect("injected soak session must build")
}

async fn wait_for_counter(counter: &AtomicUsize, target: usize, description: &str) {
    tokio::time::timeout(LIVENESS_WATCHDOG, async {
        while counter.load(Ordering::SeqCst) < target {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap_or_else(|_| panic!("timed out waiting for {description}: target {target}"));
}

async fn wait_for_observed(control: &GatedControl, target: usize, description: &str) {
    tokio::time::timeout(LIVENESS_WATCHDOG, async {
        while control.observed_application_frames() < target {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap_or_else(|_| panic!("timed out waiting for {description}: target {target}"));
}

fn assert_admission_rejection(error: &ZaiError) {
    assert!(
        error.message().contains("outbound admission timed out"),
        "unexpected fail-fast admission error: {error}"
    );
}

fn duration_micros(duration: Duration) -> u64 {
    u64::try_from(duration.as_micros()).unwrap_or(u64::MAX)
}

fn distribution(values: &[u64]) -> Value {
    assert!(
        !values.is_empty(),
        "a recorded distribution must not be empty"
    );
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    let percentile = |percent: usize| {
        let rank = sorted.len().saturating_mul(percent).div_ceil(100);
        sorted[rank.saturating_sub(1).min(sorted.len() - 1)]
    };
    json!({
        "count": sorted.len(),
        "p50_us": percentile(50),
        "p95_us": percentile(95),
        "p99_us": percentile(99),
        "max_us": sorted[sorted.len() - 1],
    })
}

fn configured_soak_seconds() -> u64 {
    let seconds = match std::env::var(SOAK_SECONDS_ENV) {
        Ok(raw) => raw
            .parse::<u64>()
            .unwrap_or_else(|_| panic!("{SOAK_SECONDS_ENV} must be an integer number of seconds")),
        Err(std::env::VarError::NotPresent) => DEFAULT_SOAK_SECONDS,
        Err(error) => panic!("could not read {SOAK_SECONDS_ENV}: {error}"),
    };
    assert!(
        (1..=MAX_SOAK_SECONDS).contains(&seconds),
        "{SOAK_SECONDS_ENV} must be in 1..={MAX_SOAK_SECONDS}"
    );
    seconds
}

#[cfg(target_os = "linux")]
fn vm_hwm_kib() -> Option<u64> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    status.lines().find_map(|line| {
        let value = line.strip_prefix("VmHWM:")?.trim();
        value.strip_suffix("kB")?.trim().parse().ok()
    })
}

#[cfg(not(target_os = "linux"))]
fn vm_hwm_kib() -> Option<u64> {
    None
}

#[derive(Debug)]
struct FeedbackReport {
    application_types: Vec<&'static str>,
    pong_rtt_us: Vec<u64>,
    pongs_before_first_application: usize,
    duplicate_pongs: usize,
}

struct FeedbackServer {
    url: String,
    join: Option<JoinHandle<Result<FeedbackReport, String>>>,
    first_ping_rx: Option<oneshot::Receiver<()>>,
    start_feedback_tx: Option<oneshot::Sender<()>>,
}

impl FeedbackServer {
    async fn start(expected_application_frames: usize) -> Self {
        let listener = TcpListener::bind(("127.0.0.1", 0))
            .await
            .expect("feedback server must bind loopback");
        let address = listener
            .local_addr()
            .expect("feedback server must expose its address");
        let (first_ping_tx, first_ping_rx) = oneshot::channel();
        let (start_feedback_tx, start_feedback_rx) = oneshot::channel();
        let join = tokio::spawn(run_feedback_server(
            listener,
            expected_application_frames,
            first_ping_tx,
            start_feedback_rx,
        ));
        Self {
            url: format!("ws://{address}"),
            join: Some(join),
            first_ping_rx: Some(first_ping_rx),
            start_feedback_tx: Some(start_feedback_tx),
        }
    }

    async fn wait_for_first_ping(&mut self) {
        let first_ping = self
            .first_ping_rx
            .take()
            .expect("first-Ping barrier must only be awaited once");
        tokio::time::timeout(LIVENESS_WATCHDOG, first_ping)
            .await
            .expect("feedback server did not send its first Ping in time")
            .expect("feedback server stopped before sending its first Ping");
    }

    fn start_feedback(&mut self) {
        self.start_feedback_tx
            .take()
            .expect("feedback barrier must only be released once")
            .send(())
            .expect("feedback server stopped before the admission barrier");
    }

    async fn finish(mut self) -> Result<FeedbackReport, String> {
        let mut join = self
            .join
            .take()
            .expect("feedback server task must be owned until finish");
        match tokio::time::timeout(LIVENESS_WATCHDOG, &mut join).await {
            Ok(result) => {
                result.map_err(|error| format!("feedback server task panicked: {error}"))?
            },
            Err(_) => {
                join.abort();
                let _ = join.await;
                Err("feedback server task exceeded its liveness watchdog".to_owned())
            },
        }
    }
}

impl Drop for FeedbackServer {
    fn drop(&mut self) {
        if let Some(join) = self.join.take() {
            join.abort();
        }
    }
}

async fn run_feedback_server(
    listener: TcpListener,
    expected_application_frames: usize,
    first_ping_tx: oneshot::Sender<()>,
    start_feedback_rx: oneshot::Receiver<()>,
) -> Result<FeedbackReport, String> {
    let (stream, _) = tokio::time::timeout(LIVENESS_WATCHDOG, listener.accept())
        .await
        .map_err(|_| "feedback server timed out accepting the client".to_owned())?
        .map_err(|error| format!("feedback server accept failed: {error}"))?;
    let mut socket = tokio_tungstenite::accept_async(stream)
        .await
        .map_err(|error| format!("feedback WebSocket handshake failed: {error}"))?;

    let init = next_ws_message(&mut socket, "session.update").await?;
    let Message::Text(init) = init else {
        return Err("feedback server expected text session.update".to_owned());
    };
    if frame_kind(init.as_str()) != "session.update" {
        return Err("feedback server did not receive session.update first".to_owned());
    }

    let mut next_sequence = 0_u32;
    socket
        .send(Message::Ping(next_sequence.to_be_bytes().to_vec().into()))
        .await
        .map_err(|error| format!("feedback server could not send first Ping: {error}"))?;
    first_ping_tx
        .send(())
        .map_err(|()| "feedback client dropped the first-Ping barrier".to_owned())?;
    tokio::time::timeout(LIVENESS_WATCHDOG, start_feedback_rx)
        .await
        .map_err(|_| "feedback admission barrier exceeded its liveness watchdog".to_owned())?
        .map_err(|_| "feedback client dropped the admission barrier".to_owned())?;
    let mut last_ping = Instant::now();
    next_sequence += 1;

    let mut application_types = Vec::with_capacity(expected_application_frames);
    let mut pong_rtt_us = Vec::with_capacity(FEEDBACK_PONG_SAMPLES);
    let mut pongs_seen = 0_usize;
    let mut last_acknowledged_payload: Option<[u8; 4]> = None;
    let mut duplicate_pongs = 0_usize;
    let mut pongs_before_first_application = None;
    let mut feedback_active = true;

    while application_types.len() < expected_application_frames || feedback_active {
        match next_ws_message(&mut socket, "feedback Pong or application frame").await? {
            Message::Pong(payload) => {
                let expected_payload = next_sequence.wrapping_sub(1).to_be_bytes();
                if last_acknowledged_payload
                    .as_ref()
                    .is_some_and(|last| payload.as_ref() == last)
                {
                    // Keep the feedback sequence stable so the report can
                    // expose any regression that emits a manual Pong in
                    // addition to tungstenite's automatic response.
                    duplicate_pongs += 1;
                    continue;
                }
                if payload.as_ref() != expected_payload {
                    return Err("feedback Pong did not echo the latest Ping payload".to_owned());
                }
                last_acknowledged_payload = Some(expected_payload);
                pongs_seen += 1;
                // The first Ping intentionally spans the producer admission
                // barrier. Exclude only that synthetic interval; all
                // immediately-fed follow-up round trips remain in the trend.
                if pongs_seen > 1 {
                    pong_rtt_us.push(duration_micros(last_ping.elapsed()));
                }
                let enough_samples = pong_rtt_us.len() >= FEEDBACK_PONG_SAMPLES;
                let application_progressed = !application_types.is_empty();
                if enough_samples && application_progressed {
                    feedback_active = false;
                } else {
                    if pongs_seen >= FEEDBACK_STARVATION_LIMIT && !application_progressed {
                        return Err(format!(
                            "application data was starved by {pongs_seen} self-feeding Pings"
                        ));
                    }
                    socket
                        .send(Message::Ping(next_sequence.to_be_bytes().to_vec().into()))
                        .await
                        .map_err(|error| {
                            format!("feedback server could not send follow-up Ping: {error}")
                        })?;
                    last_ping = Instant::now();
                    next_sequence = next_sequence.wrapping_add(1);
                    // The next Ping is already flushed, so this remains a
                    // self-feeding peer. Yielding only gives the application
                    // producer a deterministic opportunity to enqueue data;
                    // otherwise a loopback socket can complete tens of
                    // thousands of round trips before the test task is
                    // scheduled once on a busy runner.
                    tokio::task::yield_now().await;
                }
            },
            Message::Text(frame) => {
                let kind = frame_kind(frame.as_str());
                if kind == "unknown" || kind == "session.update" {
                    return Err(format!("unexpected feedback application frame: {kind}"));
                }
                if pongs_before_first_application.is_none() {
                    pongs_before_first_application = Some(pongs_seen);
                }
                application_types.push(kind);
                if pong_rtt_us.len() >= FEEDBACK_PONG_SAMPLES {
                    feedback_active = false;
                }
            },
            Message::Close(_) => {
                return Err("feedback client closed before the evidence completed".to_owned());
            },
            Message::Binary(_) | Message::Ping(_) | Message::Frame(_) => {
                return Err("feedback server received an unexpected frame kind".to_owned());
            },
        }
    }

    socket
        .send(Message::Close(None))
        .await
        .map_err(|error| format!("feedback server close failed: {error}"))?;
    Ok(FeedbackReport {
        application_types,
        pong_rtt_us,
        pongs_before_first_application: pongs_before_first_application
            .ok_or_else(|| "feedback server observed no application data".to_owned())?,
        duplicate_pongs,
    })
}

async fn next_ws_message<S>(
    socket: &mut tokio_tungstenite::WebSocketStream<S>,
    description: &str,
) -> Result<Message, String>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    tokio::time::timeout(LIVENESS_WATCHDOG, socket.next())
        .await
        .map_err(|_| format!("timed out waiting for {description}"))?
        .ok_or_else(|| format!("socket ended while waiting for {description}"))?
        .map_err(|error| format!("WebSocket read failed while waiting for {description}: {error}"))
}

async fn feedback_ping_phase() -> FeedbackReport {
    let expected_application_frames = FEEDBACK_AUDIO_FRAMES + 3;
    let mut server = FeedbackServer::start(expected_application_frames).await;
    let endpoints = EndpointConfig::builder()
        .realtime(format!("{}/realtime", server.url))
        .build(true)
        .expect("loopback feedback endpoint must be valid");
    let session = RealtimeClient::new(TEST_KEY)
        .with_endpoint_config(endpoints)
        .session(GLM_realtime_flash {})
        .input_audio_format(InputAudioFormat::Pcm16)
        .with_transport_config(transport_config())
        .build()
        .await
        .expect("feedback session must connect");

    let pcm = Bytes::from(vec![0_u8; PCM_BYTES_PER_20_MS]);
    server.wait_for_first_ping().await;
    session
        .send_audio(pcm.clone())
        .await
        .expect("the first feedback audio frame must be admitted before Ping feedback starts");
    server.start_feedback();

    let mut ticker = tokio::time::interval_at(Instant::now() + FRAME_PERIOD, FRAME_PERIOD);
    ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);
    for _ in 1..FEEDBACK_AUDIO_FRAMES {
        ticker.tick().await;
        session
            .send_audio(pcm.clone())
            .await
            .expect("feedback audio frame must be admitted");
    }
    session
        .commit_audio()
        .await
        .expect("commit must be admitted");
    session
        .create_response()
        .await
        .expect("response.create must be admitted");
    session.cancel().await.expect("cancel must be admitted");

    let report = server
        .finish()
        .await
        .unwrap_or_else(|error| panic!("feedback server failed: {error}"));
    tokio::time::timeout(LIVENESS_WATCHDOG, session.close())
        .await
        .expect("feedback session close exceeded its liveness watchdog")
        .expect("feedback session close failed");

    assert!(report.pong_rtt_us.len() >= FEEDBACK_PONG_SAMPLES);
    assert_eq!(
        report.duplicate_pongs, 0,
        "tungstenite automatic Pong was duplicated by the SDK writer"
    );
    assert!(
        report.pongs_before_first_application < FEEDBACK_STARVATION_LIMIT,
        "application data did not progress during feedback Pings"
    );
    assert_eq!(
        report.application_types[..FEEDBACK_AUDIO_FRAMES],
        vec!["input_audio_buffer.append"; FEEDBACK_AUDIO_FRAMES]
    );
    assert_eq!(
        report.application_types[FEEDBACK_AUDIO_FRAMES..],
        [
            "input_audio_buffer.commit",
            "response.create",
            "response.cancel"
        ]
    );
    report
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "record-only Realtime soak; the weekly workflow runs the long profile"]
async fn realtime_20ms_slow_peer_soak_evidence() {
    let soak_seconds = configured_soak_seconds();
    let baseline_hwm_kib = vm_hwm_kib();
    let pcm = Bytes::from(vec![0_u8; PCM_BYTES_PER_20_MS]);
    let (control, transport) = gated_transport();
    let session = injected_session(transport).await;

    // Establish the exact message-count boundary with one send held by the
    // stopped peer and the maximum 64 commands queued behind it.
    let first_started = control.state.send_started.load(Ordering::SeqCst) + 1;
    session
        .send_audio(pcm.clone())
        .await
        .expect("the in-flight frame must be admitted");
    wait_for_counter(
        &control.state.send_started,
        first_started,
        "the stopped peer send",
    )
    .await;
    for _ in 0..MAX_QUEUE_CAPACITY {
        session
            .send_audio(pcm.clone())
            .await
            .expect("all 64 bounded queue slots must be usable");
    }
    let message_limit_error = session
        .send_audio(pcm.clone())
        .await
        .expect_err("a stopped peer admitted more than 64 queued commands");
    assert_admission_rejection(&message_limit_error);
    assert_eq!(control.observed_application_frames(), 0);

    control.set_policy(PeerPolicy::FAST);
    let message_preflight_frames = MAX_QUEUE_CAPACITY + 1;
    wait_for_observed(
        &control,
        message_preflight_frames,
        "the count-bounded backlog to drain",
    )
    .await;

    // A 4 MiB PCM append expands to about 5.6 MiB on the wire. One may be in
    // flight, but a second cannot exceed the session's fixed 8 MiB byte budget.
    control.set_policy(PeerPolicy::STOPPED);
    let large_audio = Bytes::from(vec![0_u8; RAW_MEDIA_LIMIT]);
    let large_started = control.state.send_started.load(Ordering::SeqCst) + 1;
    session
        .send_audio(large_audio.clone())
        .await
        .expect("the first maximum-size media frame must be admitted");
    wait_for_counter(
        &control.state.send_started,
        large_started,
        "the maximum-size media send",
    )
    .await;
    let byte_limit_error = session
        .send_audio(large_audio.clone())
        .await
        .expect_err("two expanded 4 MiB frames exceeded the 8 MiB byte budget");
    assert_admission_rejection(&byte_limit_error);
    control.set_policy(PeerPolicy::FAST);
    wait_for_observed(
        &control,
        message_preflight_frames + 1,
        "the first maximum-size media frame",
    )
    .await;

    // Reusing the same maximum-size operation after drain proves the byte
    // permits were reclaimed, not merely that the earlier rejection fired.
    control.set_policy(PeerPolicy::STOPPED);
    let reclaimed_started = control.state.send_started.load(Ordering::SeqCst) + 1;
    session
        .send_audio(large_audio.clone())
        .await
        .expect("the reclaimed byte budget must admit another maximum frame");
    wait_for_counter(
        &control.state.send_started,
        reclaimed_started,
        "the reclaimed maximum-size media send",
    )
    .await;
    control.set_policy(PeerPolicy::FAST);
    wait_for_observed(
        &control,
        message_preflight_frames + 2,
        "the reclaimed maximum-size media frame",
    )
    .await;
    drop(large_audio);

    // Record actual 20 ms producer spacing and public admission latency while
    // the peer is consistently one millisecond slower per frame. These values
    // are trend evidence only; the exact capacity preflight above is the gate.
    control.set_policy(PeerPolicy::SLOWER_THAN_AUDIO);
    let timed_observed_base = control.observed_application_frames();
    let requested_frames = usize::try_from(
        soak_seconds
            .checked_mul(50)
            .expect("configured soak frame count overflowed"),
    )
    .expect("configured soak frame count does not fit usize");
    let mut inter_arrival_us = Vec::with_capacity(requested_frames);
    let mut admission_us = Vec::with_capacity(requested_frames);
    let mut admitted_frames = 0_usize;
    let mut rejected_frames = 0_usize;
    let mut previous_tick = Instant::now();
    let mut ticker = tokio::time::interval_at(previous_tick + FRAME_PERIOD, FRAME_PERIOD);
    ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);

    for _ in 0..requested_frames {
        ticker.tick().await;
        let tick = Instant::now();
        inter_arrival_us.push(duration_micros(tick.duration_since(previous_tick)));
        previous_tick = tick;
        let admission_started = Instant::now();
        match session.send_audio(pcm.clone()).await {
            Ok(()) => admitted_frames += 1,
            Err(error) => {
                assert_admission_rejection(&error);
                rejected_frames += 1;
            },
        }
        admission_us.push(duration_micros(admission_started.elapsed()));
    }
    assert_eq!(admitted_frames + rejected_frames, requested_frames);

    control.set_policy(PeerPolicy::FAST);
    wait_for_observed(
        &control,
        timed_observed_base + admitted_frames,
        "the timed slow-peer backlog",
    )
    .await;

    // One final stopped-peer group freezes the application ordering barrier.
    // Releasing it must preserve audio -> commit -> create -> cancel exactly.
    control.set_policy(PeerPolicy::STOPPED);
    let ordered_base = control.observed_application_frames();
    let ordered_started = control.state.send_started.load(Ordering::SeqCst) + 1;
    session
        .send_audio(pcm.clone())
        .await
        .expect("ordered audio must be admitted");
    wait_for_counter(
        &control.state.send_started,
        ordered_started,
        "the ordered stopped-peer audio send",
    )
    .await;
    session
        .commit_audio()
        .await
        .expect("commit must be admitted");
    session
        .create_response()
        .await
        .expect("response.create must be admitted");
    session.cancel().await.expect("cancel must be admitted");
    control.set_policy(PeerPolicy::FAST);
    wait_for_observed(&control, ordered_base + 4, "the ordered barrier group").await;
    let ordered = control.observed();
    let ordered_types: Vec<_> = ordered[ordered.len() - 4..]
        .iter()
        .map(|frame| frame.kind)
        .collect();
    assert_eq!(
        ordered_types,
        [
            "input_audio_buffer.append",
            "input_audio_buffer.commit",
            "response.create",
            "response.cancel"
        ]
    );

    // A background transport failure must become the observable terminal
    // stream error and the same session result, never a detached task failure.
    let mut events = session.events();
    control.fail_next_send();
    session
        .send_audio(pcm.clone())
        .await
        .expect("the synthetic failure command must first be admitted");
    let terminal_error = tokio::time::timeout(LIVENESS_WATCHDOG, events.next())
        .await
        .expect("transport failure was not surfaced within the liveness watchdog")
        .expect("event stream ended without surfacing the transport failure")
        .expect_err("transport failure was surfaced as a successful event");
    assert!(
        terminal_error
            .message()
            .contains("synthetic soak writer failure")
    );
    drop(events);
    let session_error = tokio::time::timeout(LIVENESS_WATCHDOG, session.close())
        .await
        .expect("failed session close exceeded its liveness watchdog")
        .expect_err("session close hid the background writer failure");
    assert!(
        session_error
            .message()
            .contains("synthetic soak writer failure")
    );
    assert_eq!(control.state.send_failed.load(Ordering::SeqCst), 1);
    assert_eq!(control.state.close_count.load(Ordering::SeqCst), 1);

    // Teardown has its own stopped-send fixture so the shutdown signal must
    // cancel an in-flight transport future before close can complete.
    let (close_control, close_transport) = gated_transport();
    let close_session = injected_session(close_transport).await;
    let close_started = close_control.state.send_started.load(Ordering::SeqCst) + 1;
    close_session
        .send_audio(pcm)
        .await
        .expect("blocked-close frame must be admitted");
    wait_for_counter(
        &close_control.state.send_started,
        close_started,
        "the blocked-close send",
    )
    .await;
    let close_started_at = Instant::now();
    tokio::time::timeout(LIVENESS_WATCHDOG, close_session.close())
        .await
        .expect("blocked transport close exceeded its liveness watchdog")
        .expect("blocked transport close failed");
    let blocked_close_us = duration_micros(close_started_at.elapsed());
    assert_eq!(close_control.state.close_count.load(Ordering::SeqCst), 1);

    let feedback = feedback_ping_phase().await;
    let final_hwm_kib = vm_hwm_kib();
    let hwm_delta_kib = baseline_hwm_kib
        .zip(final_hwm_kib)
        .map(|(baseline, high_water)| high_water.saturating_sub(baseline));
    let summaries = control.observed();
    let observed_wire_bytes = summaries.iter().fold(0_u64, |total, frame| {
        total.saturating_add(u64::try_from(frame.wire_bytes).unwrap_or(u64::MAX))
    });
    let max_wire_bytes = summaries
        .iter()
        .map(|frame| frame.wire_bytes)
        .max()
        .unwrap_or(0);

    let evidence = json!({
        "benchmark": "realtime_20ms_slow_peer_soak",
        "schema_version": 1,
        "profile": {
            "configured_seconds": soak_seconds,
            "frame_period_ms": FRAME_PERIOD.as_millis(),
            "pcm_bytes_per_frame": PCM_BYTES_PER_20_MS,
            "requested_frames": requested_frames,
        },
        "timed_slow_peer": {
            "admitted_frames": admitted_frames,
            "rejected_frames": rejected_frames,
            "inter_arrival": distribution(&inter_arrival_us),
            "admission": distribution(&admission_us),
        },
        "bounded_backpressure": {
            "in_flight_frames": 1,
            "queued_frames": MAX_QUEUE_CAPACITY,
            "next_message_rejected": true,
            "expanded_byte_budget_rejected_second_4mib_frame": true,
            "byte_budget_reclaimed_after_drain": true,
        },
        "transport_evidence": {
            "application_frames_observed": control.observed_application_frames(),
            "wire_bytes_observed": observed_wire_bytes,
            "max_wire_frame_bytes": max_wire_bytes,
            "ordered_audio_commit_create_cancel": true,
            "background_failure_observed": true,
            "blocked_close_us": blocked_close_us,
        },
        "feedback_ping": {
            "pong_samples": feedback.pong_rtt_us.len(),
            "duplicate_pongs": feedback.duplicate_pongs,
            "pongs_before_first_application": feedback.pongs_before_first_application,
            "pong_rtt": distribution(&feedback.pong_rtt_us),
            "application_frames": feedback.application_types.len(),
        },
        "rss": {
            "unit": "KiB",
            "baseline_hwm": baseline_hwm_kib,
            "final_hwm": final_hwm_kib,
            "hwm_delta": hwm_delta_kib,
            "gate": "record_only",
        },
    });
    println!("ZAI_REALTIME_SOAK_JSON={evidence}");
}
