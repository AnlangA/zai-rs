//! [`RealtimeSession`] — an active realtime conversation over WebSocket.
//!
//! A session owns a background event-loop task that pumps client events onto
//! the socket and fans server events (and decoded audio) out to subscribers.
//! Callers drive it via command methods (`send_audio`, `send_text`, …) and
//! consume the two streams: [`RealtimeSession::events`] and
//! [`RealtimeSession::audio_stream`].

use std::{
    future::Future,
    io::Write as _,
    pin::Pin,
    sync::{Arc, Mutex},
    time::Duration,
};

use bytes::Bytes;
use futures_util::{Stream, stream};
use tokio::{
    sync::{Semaphore, broadcast, mpsc, watch},
    task::JoinHandle,
};
use tokio_tungstenite::tungstenite::Error as WebSocketError;
use tracing::{debug, warn};

use super::{
    audio::{InputAudioFormat, OutputAudioFormat, decode_base64},
    client::AuthMode,
    config::RealtimeTransportConfig,
    events::{ClientEvent, ServerEvent, decode_server_event_compat},
    jwt,
    protocol::{
        ChatMode, GreetingConfig, InputAudioNoiseReduction, NoiseReductionType, RealtimeModality,
        RealtimeTool, RealtimeVoice, SessionConfig, TurnDetectionType,
    },
    transport::{BufferedTungsteniteTransport, RealtimeTransport, SessionFrameBudget, WsMessage},
};
use crate::{
    ZaiError, ZaiResult,
    client::{
        error::RealtimeErrorKind,
        secret::ApiSecret,
        transport::{
            limits::{REALTIME_AUDIO_FRAME_MAX, WS_MESSAGE_MAX},
            retry::{JitterSource, backoff_delay, reconcile_retry_after},
        },
    },
};

mod validation;

use validation::validate_session_config;

/// Only one caller may expand/serialize a large outbound event at a time.
/// Once the exact JSON byte budget is reserved, the next caller may prepare
/// concurrently while the prior command waits for the channel.
const OUTBOUND_PREPARATION_CAPACITY: usize = 1;
/// Aggregate bytes admitted to the regular outbound queue. A message-count
/// bound alone permits eight maximum-size WebSocket messages to retain 64 MiB;
/// the byte budget keeps the queued payload bounded to one maximum message.
const OUTBOUND_QUEUE_BYTES_MAX: usize = WS_MESSAGE_MAX as usize;
struct OutboundCommand {
    json: String,
    budget: Option<SessionFrameBudget>,
}

impl OutboundCommand {
    fn unbudgeted(json: impl Into<String>) -> Self {
        Self {
            json: json.into(),
            budget: None,
        }
    }
}

impl From<String> for OutboundCommand {
    fn from(json: String) -> Self {
        Self::unbudgeted(json)
    }
}

/// One decoded `response.audio.delta` payload with its correlation metadata.
#[derive(Debug, Clone)]
pub struct RealtimeAudioChunk {
    /// Id of the response this audio belongs to.
    pub response_id: String,
    /// Id of the output item this audio belongs to.
    pub item_id: String,
    /// Index of the output item within the response, when supplied.
    pub output_index: Option<u64>,
    /// Index of the content part within the output item, when supplied.
    pub content_index: Option<u64>,
    /// Decoded raw 24 kHz, mono, 16-bit PCM bytes.
    pub data: Bytes,
}

/// Builder for an [`RealtimeSession`].
///
/// Produced by [`super::client::RealtimeClient::session`]. Configure the
/// session defaults, then [`SessionBuilder::build`] opens the WebSocket and
/// sends the initial `session.update`.
pub struct SessionBuilder {
    api_key: Arc<ApiSecret>,
    auth: AuthMode,
    realtime_url: String,
    model_name: String,
    session_config: SessionConfig,
    transport_config: RealtimeTransportConfig,
}

impl SessionBuilder {
    pub(super) fn new(
        api_key: Arc<ApiSecret>,
        auth: AuthMode,
        realtime_url: String,
        model_name: String,
        transport_config: RealtimeTransportConfig,
    ) -> Self {
        Self {
            api_key,
            auth,
            realtime_url,
            model_name,
            session_config: SessionConfig::default(),
            transport_config,
        }
    }

    /// System instructions guiding the model.
    pub fn instructions(mut self, instructions: impl Into<String>) -> Self {
        self.session_config.instructions = Some(instructions.into());
        self
    }

    /// VAD strategy (defaults to client-VAD).
    pub fn turn_detection(mut self, vad: TurnDetectionType) -> Self {
        self.session_config.turn_detection.type_ = vad;
        self
    }

    /// Configure whether server VAD automatically creates a response at the
    /// end of a detected speech turn.
    pub fn create_response_on_vad(mut self, enabled: bool) -> Self {
        self.session_config.turn_detection.create_response = Some(enabled);
        self
    }

    /// Configure whether server VAD interrupts an in-progress response when
    /// new speech begins.
    pub fn interrupt_response_on_vad(mut self, enabled: bool) -> Self {
        self.session_config.turn_detection.interrupt_response = Some(enabled);
        self
    }

    /// Configure the server-VAD activation threshold (`0.0..=1.0`).
    pub fn vad_threshold(mut self, threshold: f64) -> Self {
        self.session_config.turn_detection.threshold = Some(threshold);
        self
    }

    /// Configure how much audio before detected speech is retained.
    pub fn vad_prefix_padding_ms(mut self, milliseconds: u32) -> Self {
        self.session_config.turn_detection.prefix_padding_ms = Some(milliseconds);
        self
    }

    /// Configure how much silence ends a server-VAD turn.
    pub fn vad_silence_duration_ms(mut self, milliseconds: u32) -> Self {
        self.session_config.turn_detection.silence_duration_ms = Some(milliseconds);
        self
    }

    /// Input audio format (defaults to 16 kHz WAV).
    pub fn input_audio_format(mut self, format: InputAudioFormat) -> Self {
        self.session_config.input_audio_format = format;
        self
    }

    /// Output audio format (defaults to PCM).
    pub fn output_audio_format(mut self, format: OutputAudioFormat) -> Self {
        self.session_config.output_audio_format = format;
        self
    }

    /// Output modalities (text, audio, or both).
    pub fn modalities(mut self, modalities: impl IntoIterator<Item = RealtimeModality>) -> Self {
        self.session_config.modalities = modalities.into_iter().collect();
        self
    }

    /// Voice used for generated audio.
    pub fn voice(mut self, voice: RealtimeVoice) -> Self {
        self.session_config.voice = Some(voice);
        self
    }

    /// Sampling temperature. Values outside `0.0..=1.0` are rejected by
    /// [`Self::build`] before a connection is opened.
    pub fn temperature(mut self, temperature: f64) -> Self {
        self.session_config.temperature = Some(temperature);
        self
    }

    /// Maximum response text-token count. Values above 1024 are rejected by
    /// [`Self::build`] before a connection is opened.
    pub fn max_response_output_tokens(mut self, tokens: u16) -> Self {
        self.session_config.max_response_output_tokens = Some(tokens);
        self
    }

    /// Configure input-audio noise reduction for the microphone placement.
    pub fn input_audio_noise_reduction(mut self, profile: NoiseReductionType) -> Self {
        self.session_config.input_audio_noise_reduction =
            Some(InputAudioNoiseReduction::new(profile));
        self
    }

    /// Conversation mode under `beta_fields.chat_mode`.
    pub fn chat_mode(mut self, mode: ChatMode) -> Self {
        self.session_config.beta_fields.chat_mode = Some(mode);
        self
    }

    /// Enable/disable the server-side built-in web search.
    pub fn auto_search(mut self, enabled: bool) -> Self {
        self.session_config.beta_fields.auto_search = Some(enabled);
        self
    }

    /// Configure an optional server-generated greeting.
    pub fn greeting_config(mut self, greeting: GreetingConfig) -> Self {
        self.session_config.greeting_config = Some(greeting);
        self
    }

    /// Register function tools.
    pub fn tools(mut self, tools: Vec<RealtimeTool>) -> Self {
        self.session_config.tools = tools;
        self
    }

    /// Override the entire session config.
    ///
    /// The model is still taken from [`RealtimeClient::session`](super::RealtimeClient::session)
    /// and replaces `config.model` during [`Self::build`] or
    /// [`Self::build_with_transport`].
    pub fn session_config(mut self, config: SessionConfig) -> Self {
        self.session_config = config;
        self
    }

    /// Override the transport/session policy inherited from the client.
    ///
    /// This value is private to this builder and becomes the effective policy
    /// exposed by the resulting [`RealtimeSession`].
    pub fn with_transport_config(mut self, config: RealtimeTransportConfig) -> Self {
        self.transport_config = config;
        self
    }

    /// The effective transport/session policy for this builder.
    pub fn transport_config(&self) -> &RealtimeTransportConfig {
        &self.transport_config
    }

    /// Validate the complete session configuration and authentication inputs
    /// without opening a WebSocket.
    ///
    /// This applies the selected type-safe model, checks transport-policy and
    /// cross-field protocol rules, serializes the initial `session.update`
    /// under the realtime message limit, and validates the API key plus
    /// optional JWT lifetime. A
    /// successful result guarantees that [`build`](Self::build) can proceed to
    /// its first network operation, though the remote connection may still
    /// fail.
    pub fn validate(&self) -> ZaiResult<()> {
        self.transport_config.validate()?;
        let _ = prepare_session_update(&self.model_name, self.session_config.clone())?;
        let _ = jwt::authorization_header(self.api_key.expose(), jwt_ttl(self.auth))?;
        Ok(())
    }

    /// Open the WebSocket with the effective transport policy, confirm the
    /// initial `session.update`, and spawn the event loop.
    ///
    /// Built-in connection attempts and their backoff share one absolute
    /// [`RealtimeTransportConfig::connect_timeout`] budget. Only explicitly
    /// classified transient socket or HTTP handshake failures are retried, up
    /// to [`RealtimeTransportConfig::max_connect_attempts`]. Authorization is
    /// generated afresh for every attempt so a short-lived JWT is not reused.
    /// Once a connection succeeds, the initial `session.update` is sent exactly
    /// once: an error, timeout, or cancellation during that confirmed write is
    /// ambiguous and is never followed by an automatic reconnect or replay.
    #[tracing::instrument(name = "realtime.session.build", skip_all, fields(model = %self.model_name))]
    pub async fn build(self) -> ZaiResult<RealtimeSession> {
        let Self {
            api_key,
            auth,
            realtime_url,
            model_name,
            session_config,
            transport_config,
        } = self;

        transport_config.validate()?;
        let (init, input_audio_format) = prepare_session_update(&model_name, session_config)?;
        let mut transport =
            connect_before_session_update(api_key.as_ref(), auth, &realtime_url, &transport_config)
                .await?;

        if let Err(error) = transport.send_confirmed(init).await {
            let _ = close_transport(&mut transport, &transport_config).await;
            return Err(error);
        }
        Ok(spawn_builtin_session(
            transport.into_buffered(),
            model_name,
            input_audio_format,
            transport_config,
        ))
    }

    /// Start a session over an already-connected, already-authenticated
    /// realtime transport.
    ///
    /// The SDK validates and confirms the initial `session.update` before this
    /// method returns, but never passes the SDK-managed API key, JWT,
    /// Authorization header, or configured URL to `transport`. This makes the
    /// entry point suitable for custom runtimes, tunnels, deterministic tests,
    /// and application-managed authentication. The transport does receive the
    /// complete `session.update`, including application-provided instructions,
    /// greetings, and tool schemas; implementations must protect that payload
    /// and redact their own logs and errors.
    /// [`RealtimeClient`](super::RealtimeClient) credentials are therefore not
    /// validated or used on this path; authentication is wholly the injected
    /// transport's responsibility.
    ///
    /// [`RealtimeTransport::send_confirmed`] must not resolve until the whole
    /// initial update is written. Its default implementation delegates to
    /// [`RealtimeTransport::send`], so buffered transports must override it.
    /// Initial-send failure is not retried because a partial write is
    /// ambiguous. This method takes ownership of `transport` and attempts a
    /// bounded close after validation or initial-send failure; cleanup cannot
    /// replace the primary error.
    ///
    /// The effective [`RealtimeTransportConfig`] still governs the SDK-owned
    /// outbound admission queue, event/audio buffers, inbound-idle deadline,
    /// and outer initial/send/close deadlines. Connect, Pong, writer-queue,
    /// and frame-size settings are built-in-Tungstenite details and are not
    /// passed to or enforced on this already-connected transport.
    #[tracing::instrument(
        name = "realtime.session.build_with_transport",
        skip_all,
        fields(model = %self.model_name)
    )]
    pub async fn build_with_transport<T>(self, mut transport: T) -> ZaiResult<RealtimeSession>
    where
        T: RealtimeTransport + 'static,
    {
        let Self {
            model_name,
            session_config,
            transport_config,
            ..
        } = self;

        if let Err(error) = transport_config.validate() {
            let _ = close_transport(&mut transport, &transport_config).await;
            return Err(error);
        }
        let (init, input_audio_format) = match prepare_session_update(&model_name, session_config) {
            Ok(prepared) => prepared,
            Err(error) => {
                let _ = close_transport(&mut transport, &transport_config).await;
                return Err(error);
            },
        };
        let initial_send = tokio::time::timeout(
            transport_config.initial_update_timeout(),
            transport.send_confirmed(init),
        )
        .await
        .map_err(|_| {
            crate::ZaiError::from(RealtimeErrorKind::Timeout {
                operation: "Realtime initial session.update send",
            })
        })
        .and_then(|result| result);
        if let Err(error) = initial_send {
            let _ = close_transport(&mut transport, &transport_config).await;
            return Err(error);
        }

        Ok(spawn_injected_session(
            transport,
            model_name,
            input_audio_format,
            transport_config,
        ))
    }
}

#[async_trait::async_trait]
trait SessionTransport: Send {
    async fn send_command(&mut self, command: OutboundCommand) -> ZaiResult<()>;
    async fn recv(&mut self) -> ZaiResult<Option<WsMessage>>;
    async fn close(&mut self) -> ZaiResult<()>;
}

struct InjectedSessionTransport<T>(T);

#[async_trait::async_trait]
impl<T> SessionTransport for InjectedSessionTransport<T>
where
    T: RealtimeTransport,
{
    async fn send_command(&mut self, command: OutboundCommand) -> ZaiResult<()> {
        let OutboundCommand { json, budget } = command;
        let result = self.0.send(json).await;
        drop(budget);
        result
    }

    async fn recv(&mut self) -> ZaiResult<Option<WsMessage>> {
        self.0.recv().await
    }

    async fn close(&mut self) -> ZaiResult<()> {
        self.0.close().await
    }
}

#[async_trait::async_trait]
trait BuiltInSessionIo: Send {
    fn enqueue_session_text(&mut self, json: String, budget: SessionFrameBudget) -> ZaiResult<()>;
    async fn recv(&mut self) -> ZaiResult<Option<WsMessage>>;
    async fn close(&mut self) -> ZaiResult<()>;
}

#[async_trait::async_trait]
impl BuiltInSessionIo for BufferedTungsteniteTransport {
    fn enqueue_session_text(&mut self, json: String, budget: SessionFrameBudget) -> ZaiResult<()> {
        BufferedTungsteniteTransport::enqueue_session_text(self, json, budget)
    }

    async fn recv(&mut self) -> ZaiResult<Option<WsMessage>> {
        RealtimeTransport::recv(self).await
    }

    async fn close(&mut self) -> ZaiResult<()> {
        RealtimeTransport::close(self).await
    }
}

struct BuiltInSessionTransport<T>(T);

#[async_trait::async_trait]
impl<T> SessionTransport for BuiltInSessionTransport<T>
where
    T: BuiltInSessionIo,
{
    async fn send_command(&mut self, command: OutboundCommand) -> ZaiResult<()> {
        let OutboundCommand { json, budget } = command;
        let budget = budget.ok_or_else(|| {
            protocol_error("built-in realtime command is missing its admission budget")
        })?;
        self.0.enqueue_session_text(json, budget)
    }

    async fn recv(&mut self) -> ZaiResult<Option<WsMessage>> {
        self.0.recv().await
    }

    async fn close(&mut self) -> ZaiResult<()> {
        self.0.close().await
    }
}

fn spawn_injected_session<T>(
    transport: T,
    model_name: String,
    input_audio_format: InputAudioFormat,
    transport_config: RealtimeTransportConfig,
) -> RealtimeSession
where
    T: RealtimeTransport + 'static,
{
    spawn_session(
        InjectedSessionTransport(transport),
        model_name,
        input_audio_format,
        transport_config,
        None,
    )
}

fn spawn_builtin_session(
    transport: BufferedTungsteniteTransport,
    model_name: String,
    input_audio_format: InputAudioFormat,
    transport_config: RealtimeTransportConfig,
) -> RealtimeSession {
    // Count the complete built-in pipeline rather than treating the session
    // and writer queues as independent capacity pools. The smaller configured
    // bound wins, and its permit follows the command until the sink finishes.
    let pipeline_capacity = built_in_pipeline_capacity(&transport_config);
    spawn_session(
        BuiltInSessionTransport(transport),
        model_name,
        input_audio_format,
        transport_config,
        Some(pipeline_capacity),
    )
}

fn built_in_pipeline_capacity(config: &RealtimeTransportConfig) -> usize {
    config
        .outbound_queue_capacity()
        .min(config.writer_queue_capacity())
}

fn spawn_session<T>(
    transport: T,
    model_name: String,
    input_audio_format: InputAudioFormat,
    transport_config: RealtimeTransportConfig,
    pipeline_capacity: Option<usize>,
) -> RealtimeSession
where
    T: SessionTransport + 'static,
{
    debug!(model = %model_name, "Realtime session opened");
    let (cmd_tx, cmd_rx) =
        mpsc::channel::<OutboundCommand>(transport_config.outbound_queue_capacity());
    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    let (events_tx, _) =
        broadcast::channel::<ServerEvent>(transport_config.event_buffer_capacity());
    let (audio_tx, _) =
        broadcast::channel::<RealtimeAudioChunk>(transport_config.audio_buffer_capacity());
    // Subscribe before the event loop starts so session-created events and
    // greeting audio cannot race ahead of the caller's first subscription.
    let initial_events_rx = events_tx.subscribe();
    let initial_audio_rx = audio_tx.subscribe();

    let (completion_tx, completion_rx) = watch::channel(None);
    let loop_events_tx = events_tx.clone();
    let loop_audio_tx = audio_tx.clone();
    let loop_transport_config = transport_config.clone();
    let join = tokio::spawn(async move {
        let result = run_session_loop(
            transport,
            cmd_rx,
            shutdown_rx,
            loop_events_tx,
            loop_audio_tx,
            loop_transport_config,
        )
        .await;
        completion_tx.send_replace(Some(result.clone()));
        result
    });

    RealtimeSession {
        cmd_tx,
        outbound_budget: Arc::new(Semaphore::new(OUTBOUND_QUEUE_BYTES_MAX)),
        outbound_slots: pipeline_capacity.map(|capacity| Arc::new(Semaphore::new(capacity))),
        outbound_preparation: Arc::new(Semaphore::new(OUTBOUND_PREPARATION_CAPACITY)),
        shutdown_tx,
        events_tx,
        audio_tx,
        initial_events_rx: Mutex::new(Some(initial_events_rx)),
        initial_audio_rx: Mutex::new(Some(initial_audio_rx)),
        completion_rx,
        model_name,
        input_audio_format,
        transport_config,
        join,
    }
}

/// Background event-loop body: drains commands onto the socket and fans server
/// messages out to the broadcast channels. Generic over the transport so a mock
/// can be substituted in tests.
#[cfg(test)]
async fn run_loop<T, C>(
    transport: T,
    cmd_rx: mpsc::Receiver<C>,
    shutdown_rx: watch::Receiver<bool>,
    events_tx: broadcast::Sender<ServerEvent>,
    audio_tx: broadcast::Sender<RealtimeAudioChunk>,
    transport_config: RealtimeTransportConfig,
) -> ZaiResult<()>
where
    T: RealtimeTransport,
    C: Into<OutboundCommand>,
{
    run_session_loop(
        InjectedSessionTransport(transport),
        cmd_rx,
        shutdown_rx,
        events_tx,
        audio_tx,
        transport_config,
    )
    .await
}

async fn run_session_loop<T, C>(
    mut transport: T,
    mut cmd_rx: mpsc::Receiver<C>,
    mut shutdown_rx: watch::Receiver<bool>,
    events_tx: broadcast::Sender<ServerEvent>,
    audio_tx: broadcast::Sender<RealtimeAudioChunk>,
    transport_config: RealtimeTransportConfig,
) -> ZaiResult<()>
where
    T: SessionTransport,
    C: Into<OutboundCommand>,
{
    let inbound_idle_timeout = transport_config.inbound_idle_timeout();
    let idle_deadline = tokio::time::sleep(inbound_idle_timeout);
    tokio::pin!(idle_deadline);

    loop {
        tokio::select! {
            biased;
            changed = shutdown_rx.changed() => {
                if changed.is_err() || *shutdown_rx.borrow() {
                    debug!("Realtime session closed (client requested)");
                    return close_session_transport(&mut transport, &transport_config).await;
                }
            },
            // Poll inbound traffic before the deadline so a heartbeat arriving
            // exactly at the boundary refreshes the session instead of racing
            // with a false timeout. One queued command is drained after every
            // inbound frame, preventing either direction from starving the
            // other under sustained traffic.
            msg = transport.recv() => match msg {
                Ok(Some(WsMessage::Text(text))) => {
                    idle_deadline
                        .as_mut()
                        .reset(tokio::time::Instant::now() + inbound_idle_timeout);
                    match decode_server_frame(&text) {
                        Ok(DecodedServerFrame::Event(event)) => {
                            if let ServerEvent::Error { error } = event.as_ref() {
                                // The free-form message may echo caller content, so
                                // only the machine-readable code enters logs.
                                warn!(code = ?error.code, "Realtime server error event");
                            }
                            let _ = events_tx.send(*event);
                        },
                        Ok(DecodedServerFrame::Audio(bytes)) => {
                            let _ = audio_tx.send(bytes);
                        },
                        Ok(DecodedServerFrame::Unknown) => {
                            warn!(bytes = text.len(), "Ignoring unknown realtime event");
                        },
                        Err(error) => {
                            warn!(bytes = text.len(), "Closing session after malformed realtime event");
                            let _ = close_session_transport(&mut transport, &transport_config).await;
                            return Err(error);
                        },
                    }

                    match cmd_rx.try_recv() {
                        Ok(command) => {
                            if !handle_outbound(
                                &mut transport,
                                Some(command.into()),
                                &mut shutdown_rx,
                                &transport_config,
                            )
                            .await?
                            {
                                return Ok(());
                            }
                        },
                        Err(mpsc::error::TryRecvError::Disconnected) => {
                            handle_outbound(
                                &mut transport,
                                None,
                                &mut shutdown_rx,
                                &transport_config,
                            )
                            .await?;
                            return Ok(());
                        },
                        Err(mpsc::error::TryRecvError::Empty) => {},
                    }
                },
                Ok(Some(WsMessage::Binary(bytes))) => {
                    warn!(bytes = bytes.len(), "Closing session after unexpected realtime binary frame");
                    let _ = close_session_transport(&mut transport, &transport_config).await;
                    return Err(protocol_error(
                        "unexpected binary frame in realtime JSON protocol",
                    ));
                },
                Ok(None) => {
                    debug!("Realtime session closed (peer disconnected)");
                    // The built-in transport owns an independent writer task;
                    // join its shutdown even though the peer close itself is a
                    // clean session outcome.
                    return close_session_transport(&mut transport, &transport_config).await;
                },
                Err(error) => {
                    // Avoid logging the error source: handshake/transport errors
                    // can contain endpoint details or server-provided text.
                    warn!("Realtime event loop terminated due to transport error");
                    let _ = close_session_transport(&mut transport, &transport_config).await;
                    return Err(error);
                },
            },
            _ = &mut idle_deadline => {
                warn!(
                    timeout_seconds = inbound_idle_timeout.as_secs(),
                    "Realtime session timed out waiting for inbound traffic"
                );
                let _ = close_session_transport(&mut transport, &transport_config).await;
                return Err(RealtimeErrorKind::Timeout {
                    operation: "Realtime inbound heartbeat",
                }
                .into());
            },
            cmd = cmd_rx.recv() => {
                if !handle_outbound(
                    &mut transport,
                    cmd.map(Into::into),
                    &mut shutdown_rx,
                    &transport_config,
                )
                .await?
                {
                    return Ok(());
                }
            },
        }
    }
}

enum DecodedServerFrame {
    Event(Box<ServerEvent>),
    Audio(RealtimeAudioChunk),
    Unknown,
}

fn decode_server_frame(text: &str) -> ZaiResult<DecodedServerFrame> {
    if text.len() as u64 > WS_MESSAGE_MAX {
        return Err(protocol_error(format!(
            "realtime inbound message exceeds {WS_MESSAGE_MAX} bytes"
        )));
    }
    let event = decode_server_event_compat(text)
        .map_err(|_| protocol_error("malformed realtime server event"))?;
    match event {
        ServerEvent::ResponseAudioDelta {
            response_id,
            item_id,
            output_index,
            content_index,
            delta,
        } => {
            let bytes = decode_audio_delta(&delta)?;
            Ok(DecodedServerFrame::Audio(RealtimeAudioChunk {
                response_id,
                item_id,
                output_index,
                content_index,
                data: Bytes::from(bytes),
            }))
        },
        ServerEvent::Unknown => Ok(DecodedServerFrame::Unknown),
        event => Ok(DecodedServerFrame::Event(Box::new(event))),
    }
}

fn decode_audio_delta(delta: &str) -> ZaiResult<Vec<u8>> {
    let encoded_max = base64::encoded_len(REALTIME_AUDIO_FRAME_MAX as usize, true)
        .ok_or_else(|| protocol_error("realtime audio encoded-length overflow"))?;
    if delta.len() > encoded_max {
        return Err(protocol_error(format!(
            "encoded realtime audio delta exceeds the {REALTIME_AUDIO_FRAME_MAX}-byte decoded limit"
        )));
    }
    let bytes = decode_base64(delta)?;
    if bytes.len() as u64 > REALTIME_AUDIO_FRAME_MAX {
        return Err(protocol_error(format!(
            "realtime audio delta exceeds {REALTIME_AUDIO_FRAME_MAX} bytes"
        )));
    }
    Ok(bytes)
}

async fn handle_outbound<T: SessionTransport>(
    transport: &mut T,
    message: Option<OutboundCommand>,
    shutdown_rx: &mut watch::Receiver<bool>,
    transport_config: &RealtimeTransportConfig,
) -> ZaiResult<bool> {
    match message {
        Some(command) => {
            enum SendOutcome {
                Sent(ZaiResult<()>),
                Shutdown,
            }
            // A third-party transport may block inside `send`. The dedicated
            // shutdown channel must still be able to cancel that future so
            // `close()` never waits for an entire media backlog or write
            // timeout. A hard deadline also prevents a third-party send from
            // suspending inbound processing indefinitely. The built-in
            // transport's send path is cancellation-safe because it only
            // admits a frame to its bounded writer queue.
            let outcome = {
                let send = tokio::time::timeout(
                    transport_config.transport_send_timeout(),
                    transport.send_command(command),
                );
                tokio::pin!(send);
                loop {
                    tokio::select! {
                        biased;
                        changed = shutdown_rx.changed() => {
                            if changed.is_err() || *shutdown_rx.borrow() {
                                break SendOutcome::Shutdown;
                            }
                        },
                        result = &mut send => {
                            let result = result
                                .map_err(|_| {
                                    crate::ZaiError::from(RealtimeErrorKind::Timeout {
                                        operation: "Realtime transport send",
                                    })
                                })
                                .and_then(|result| result);
                            break SendOutcome::Sent(result);
                        },
                    }
                }
            };

            match outcome {
                SendOutcome::Sent(Ok(())) => Ok(true),
                SendOutcome::Sent(Err(error)) => {
                    let _ = close_session_transport(transport, transport_config).await;
                    Err(error)
                },
                SendOutcome::Shutdown => {
                    debug!("Realtime session closed (client requested)");
                    close_session_transport(transport, transport_config).await?;
                    Ok(false)
                },
            }
        },
        None => {
            debug!("Realtime session closed (client requested)");
            close_session_transport(transport, transport_config).await?;
            Ok(false)
        },
    }
}

async fn close_session_transport<T: SessionTransport>(
    transport: &mut T,
    transport_config: &RealtimeTransportConfig,
) -> ZaiResult<()> {
    tokio::time::timeout(
        transport_config.transport_close_timeout(),
        transport.close(),
    )
    .await
    .map_err(|_| RealtimeErrorKind::Timeout {
        operation: "Realtime transport close",
    })?
}

async fn close_transport<T: RealtimeTransport>(
    transport: &mut T,
    transport_config: &RealtimeTransportConfig,
) -> ZaiResult<()> {
    tokio::time::timeout(
        transport_config.transport_close_timeout(),
        transport.close(),
    )
    .await
    .map_err(|_| RealtimeErrorKind::Timeout {
        operation: "Realtime transport close",
    })?
}

/// An active realtime session.
///
/// Cheap to share indirectly via the channels it owns; call
/// [`RealtimeSession::close`] to terminate the background task.
pub struct RealtimeSession {
    cmd_tx: mpsc::Sender<OutboundCommand>,
    outbound_budget: Arc<Semaphore>,
    outbound_slots: Option<Arc<Semaphore>>,
    outbound_preparation: Arc<Semaphore>,
    shutdown_tx: watch::Sender<bool>,
    events_tx: broadcast::Sender<ServerEvent>,
    audio_tx: broadcast::Sender<RealtimeAudioChunk>,
    initial_events_rx: Mutex<Option<broadcast::Receiver<ServerEvent>>>,
    initial_audio_rx: Mutex<Option<broadcast::Receiver<RealtimeAudioChunk>>>,
    completion_rx: watch::Receiver<Option<ZaiResult<()>>>,
    model_name: String,
    input_audio_format: InputAudioFormat,
    transport_config: RealtimeTransportConfig,
    join: JoinHandle<ZaiResult<()>>,
}

impl RealtimeSession {
    /// Send raw 16-bit little-endian mono PCM.
    ///
    /// With [`InputAudioFormat::Wav`] (the default), the bytes are wrapped in a
    /// 16 kHz WAV container. With `Pcm16` or `Pcm24`, they are sent as raw PCM
    /// and the selected format declares the corresponding sample rate.
    pub async fn send_audio(&self, pcm: Bytes) -> ZaiResult<()> {
        if pcm.is_empty() {
            return Err(protocol_error("realtime audio frame must not be empty"));
        }
        if pcm.len() as u64 > REALTIME_AUDIO_FRAME_MAX {
            return Err(protocol_error(format!(
                "realtime audio frame exceeds {REALTIME_AUDIO_FRAME_MAX} bytes"
            )));
        }
        if !pcm.len().is_multiple_of(2) {
            return Err(protocol_error(
                "16-bit PCM input must contain an even number of bytes",
            ));
        }
        let input_audio_format = self.input_audio_format;
        self.prepare_serialized_and_dispatch(move || {
            serialize_audio_append(&pcm, input_audio_format, Some(now_ms()))
        })
        .await
    }

    /// Upload a JPEG frame for passive-video mode.
    pub async fn send_video_frame(&self, jpeg: Bytes) -> ZaiResult<()> {
        if jpeg.len() as u64 > REALTIME_AUDIO_FRAME_MAX {
            return Err(protocol_error(format!(
                "realtime video frame exceeds {REALTIME_AUDIO_FRAME_MAX} bytes"
            )));
        }
        if !jpeg.starts_with(&[0xff, 0xd8]) || !jpeg.ends_with(&[0xff, 0xd9]) {
            return Err(protocol_error(
                "realtime video frame must be a complete JPEG image",
            ));
        }
        self.prepare_serialized_and_dispatch(move || {
            serialize_video_frame_append(&jpeg, Some(now_ms()))
        })
        .await
    }

    /// Commit buffered audio for inference in client-VAD mode. Server-VAD
    /// commits automatically and normally does not need this command.
    pub async fn commit_audio(&self) -> ZaiResult<()> {
        self.dispatch(ClientEvent::InputAudioBufferCommit {
            client_timestamp: Some(now_ms()),
        })
        .await
    }

    /// Clear audio buffered by the server without triggering inference.
    pub async fn clear_audio(&self) -> ZaiResult<()> {
        self.dispatch(ClientEvent::InputAudioBufferClear).await
    }

    /// Inject a user text message into the conversation history.
    pub async fn send_text(&self, text: impl Into<String>) -> ZaiResult<()> {
        let text = text.into();
        if text.trim().is_empty() {
            return Err(protocol_error("realtime text must not be blank"));
        }
        self.dispatch(ClientEvent::ConversationItemCreate {
            event_id: Some(new_event_id()),
            item: super::protocol::RealtimeConversationItem::user_text(text),
        })
        .await
    }

    /// Feed back a function-call result.
    pub async fn send_function_output(
        &self,
        call_name: impl Into<String>,
        output: impl Into<String>,
    ) -> ZaiResult<()> {
        let call_name = call_name.into();
        if call_name.trim().is_empty() {
            return Err(protocol_error("realtime function name must not be blank"));
        }
        let output = output.into();
        if output.trim().is_empty() {
            return Err(protocol_error("realtime function output must not be blank"));
        }
        self.dispatch(ClientEvent::ConversationItemCreate {
            event_id: Some(new_event_id()),
            item: super::protocol::RealtimeConversationItem::function_output(call_name, output),
        })
        .await
    }

    /// Delete an item from the server-side conversation history.
    pub async fn delete_item(&self, item_id: impl Into<String>) -> ZaiResult<()> {
        let item_id = item_id.into();
        if item_id.trim().is_empty() {
            return Err(protocol_error("realtime item id must not be blank"));
        }
        self.dispatch(ClientEvent::ConversationItemDelete {
            event_id: Some(new_event_id()),
            client_timestamp: Some(now_ms()),
            item_id,
        })
        .await
    }

    /// Ask the server to emit the current representation of one conversation
    /// item via [`ServerEvent::ConversationItemRetrieved`].
    pub async fn retrieve_item(&self, item_id: impl Into<String>) -> ZaiResult<()> {
        let item_id = item_id.into();
        if item_id.trim().is_empty() {
            return Err(protocol_error("realtime item id must not be blank"));
        }
        self.dispatch(ClientEvent::ConversationItemRetrieve {
            event_id: Some(new_event_id()),
            client_timestamp: Some(now_ms()),
            item_id,
        })
        .await
    }

    /// Trigger model inference (`response.create`).
    pub async fn create_response(&self) -> ZaiResult<()> {
        self.dispatch(ClientEvent::ResponseCreate {
            client_timestamp: Some(now_ms()),
        })
        .await
    }

    /// Cancel the in-flight response (`response.cancel`), e.g. on interruption.
    ///
    /// The event shares the ordered application queue, so an awaited
    /// `create_response()` or audio commit is always written before its later
    /// cancellation.
    pub async fn cancel(&self) -> ZaiResult<()> {
        self.dispatch(ClientEvent::ResponseCancel {
            client_timestamp: Some(now_ms()),
        })
        .await
    }

    /// Stream of server metadata events (transcripts, response lifecycle,
    /// errors, heartbeats). Audio deltas are decoded only onto
    /// [`Self::audio_stream`] to avoid retaining a second base64 copy. The
    /// first subscriber receives events buffered since session creation;
    /// later subscribers start at the live tail. A lagged consumer or
    /// background session failure is surfaced as an error instead of silently
    /// losing protocol events.
    pub fn events(&self) -> Pin<Box<dyn Stream<Item = ZaiResult<ServerEvent>> + Send + '_>> {
        observable_broadcast_stream(
            subscribe_with_initial_backlog(&self.events_tx, &self.initial_events_rx),
            self.completion_rx.clone(),
            "realtime event",
        )
    }

    /// Stream of decoded 24 kHz, mono, 16-bit PCM output chunks.
    ///
    /// Lag is an error because dropping a PCM chunk would silently corrupt the
    /// resulting audio stream.
    pub fn audio_stream(
        &self,
    ) -> Pin<Box<dyn Stream<Item = ZaiResult<RealtimeAudioChunk>> + Send + '_>> {
        observable_broadcast_stream(
            subscribe_with_initial_backlog(&self.audio_tx, &self.initial_audio_rx),
            self.completion_rx.clone(),
            "realtime audio",
        )
    }

    /// The model id sent in the initial `session.update` event.
    pub fn model_name(&self) -> &str {
        &self.model_name
    }

    /// Complete policy captured when this session started.
    ///
    /// Built-in sessions apply every field. For an injected transport, this
    /// session applies only SDK-owned admission, buffer, idle, send, initial,
    /// and close policy; wire-side connect/Pong/writer/frame fields describe no
    /// behavior the SDK can impose on third-party code.
    pub fn transport_config(&self) -> &RealtimeTransportConfig {
        &self.transport_config
    }

    #[tracing::instrument(name = "realtime.dispatch", skip(self, event))]
    async fn dispatch(&self, event: ClientEvent) -> ZaiResult<()> {
        self.prepare_and_dispatch(|| Ok(event)).await
    }

    async fn prepare_and_dispatch<F>(&self, prepare: F) -> ZaiResult<()>
    where
        F: FnOnce() -> ZaiResult<ClientEvent>,
    {
        self.prepare_wire_and_dispatch(move |deadline| {
            let event = prepare()?;
            if let Some(deadline) = deadline {
                ensure_outbound_deadline(deadline)?;
            }
            serialize_event(&event)
        })
        .await
    }

    async fn prepare_serialized_and_dispatch<F>(&self, prepare: F) -> ZaiResult<()>
    where
        F: FnOnce() -> ZaiResult<String>,
    {
        self.prepare_wire_and_dispatch(move |_| prepare()).await
    }

    async fn prepare_wire_and_dispatch<F>(&self, prepare: F) -> ZaiResult<()>
    where
        F: FnOnce(Option<tokio::time::Instant>) -> ZaiResult<String>,
    {
        let queue_timeout = self.transport_config.outbound_queue_timeout();
        if queue_timeout.is_zero() {
            return self.try_prepare_wire_and_dispatch(prepare);
        }

        let deadline = tokio::time::Instant::now() + queue_timeout;
        tokio::time::timeout_at(
            deadline,
            self.prepare_wire_and_dispatch_waiting(prepare, deadline),
        )
        .await
        .map_err(|_| outbound_admission_timeout())?
    }

    async fn prepare_wire_and_dispatch_waiting<F>(
        &self,
        prepare: F,
        deadline: tokio::time::Instant,
    ) -> ZaiResult<()>
    where
        F: FnOnce(Option<tokio::time::Instant>) -> ZaiResult<String>,
    {
        let admission = Arc::clone(&self.outbound_preparation)
            .acquire_owned()
            .await
            .map_err(|_| RealtimeErrorKind::Closed)?;
        ensure_outbound_deadline(deadline)?;
        let message = prepare(Some(deadline))?;
        ensure_outbound_deadline(deadline)?;
        let command = budget_outbound_command(
            Arc::clone(&self.outbound_budget),
            self.outbound_slots.clone(),
            message,
        )
        .await?;
        ensure_outbound_deadline(deadline)?;
        // Exact queued bytes are now accounted by `command`; another caller
        // may prepare while this one waits for a channel slot.
        drop(admission);
        let slot = self
            .cmd_tx
            .reserve()
            .await
            .map_err(|_| crate::ZaiError::from(RealtimeErrorKind::Closed))?;
        // Reserving capacity does not publish the command. Recheck the
        // absolute deadline before the irreversible queue side effect so a
        // slot released at the boundary cannot admit a late message.
        ensure_outbound_deadline(deadline)?;
        slot.send(command);
        Ok(())
    }

    fn try_prepare_wire_and_dispatch<F>(&self, prepare: F) -> ZaiResult<()>
    where
        F: FnOnce(Option<tokio::time::Instant>) -> ZaiResult<String>,
    {
        let admission = Arc::clone(&self.outbound_preparation)
            .try_acquire_owned()
            .map_err(map_try_admission_error)?;
        let message = prepare(None)?;
        let command = try_budget_outbound_command(
            Arc::clone(&self.outbound_budget),
            self.outbound_slots.clone(),
            message,
        )?;
        drop(admission);
        self.cmd_tx.try_send(command).map_err(|error| match error {
            mpsc::error::TrySendError::Full(_) => outbound_admission_timeout(),
            mpsc::error::TrySendError::Closed(_) => RealtimeErrorKind::Closed.into(),
        })
    }

    /// Signal the background task to close without awaiting it.
    ///
    /// Use when the session is shared (e.g. behind an `Arc`), driven from a
    /// `tokio::select!`, or closed reactively on a shutdown signal. This only
    /// signals a dedicated shutdown channel; the background loop observes it
    /// independently of the bounded outbound queue and exits.
    /// For deterministic, awaited teardown use [`RealtimeSession::close`].
    pub async fn request_close(&self) -> ZaiResult<()> {
        self.shutdown_tx
            .send(true)
            .map_err(|_| RealtimeErrorKind::Closed.into())
    }

    /// Close the session and wait for the event loop to finish.
    pub async fn close(self) -> ZaiResult<()> {
        // Best-effort: the loop may already have ended due to a peer close or
        // protocol error.
        let _ = self.shutdown_tx.send(true);
        // Preserve both task failures and transport-close failures for the
        // caller instead of converting abnormal teardown into success.
        match self.join.await {
            Ok(result) => result,
            Err(join_error) => Err(protocol_error(format!(
                "realtime event loop join failed: {join_error}"
            ))),
        }
    }
}

async fn budget_outbound_command(
    budget: Arc<Semaphore>,
    slots: Option<Arc<Semaphore>>,
    json: String,
) -> ZaiResult<OutboundCommand> {
    let permits = outbound_message_permits(&json)?;
    let byte_budget = budget
        .acquire_many_owned(permits)
        .await
        .map_err(|_| RealtimeErrorKind::Closed)?;
    let slot_budget = match slots {
        Some(slots) => Some(
            slots
                .acquire_owned()
                .await
                .map_err(|_| RealtimeErrorKind::Closed)?,
        ),
        None => None,
    };
    Ok(OutboundCommand {
        json,
        budget: Some(SessionFrameBudget::new(byte_budget, slot_budget)),
    })
}

fn try_budget_outbound_command(
    budget: Arc<Semaphore>,
    slots: Option<Arc<Semaphore>>,
    json: String,
) -> ZaiResult<OutboundCommand> {
    let permits = outbound_message_permits(&json)?;
    let byte_budget = budget
        .try_acquire_many_owned(permits)
        .map_err(map_try_admission_error)?;
    let slot_budget = slots
        .map(|slots| slots.try_acquire_owned().map_err(map_try_admission_error))
        .transpose()?;
    Ok(OutboundCommand {
        json,
        budget: Some(SessionFrameBudget::new(byte_budget, slot_budget)),
    })
}

fn outbound_message_permits(json: &str) -> ZaiResult<u32> {
    if json.len() > OUTBOUND_QUEUE_BYTES_MAX {
        return Err(protocol_error(format!(
            "realtime outbound message exceeds the {OUTBOUND_QUEUE_BYTES_MAX}-byte queue budget"
        )));
    }
    u32::try_from(json.len())
        .map_err(|_| protocol_error("realtime outbound message length overflow"))
}

fn map_try_admission_error(error: tokio::sync::TryAcquireError) -> crate::ZaiError {
    match error {
        tokio::sync::TryAcquireError::Closed => RealtimeErrorKind::Closed.into(),
        tokio::sync::TryAcquireError::NoPermits => outbound_admission_timeout(),
    }
}

fn outbound_admission_timeout() -> crate::ZaiError {
    RealtimeErrorKind::Timeout {
        operation: "Realtime outbound admission",
    }
    .into()
}

fn ensure_outbound_deadline(deadline: tokio::time::Instant) -> ZaiResult<()> {
    if tokio::time::Instant::now() >= deadline {
        return Err(outbound_admission_timeout());
    }
    Ok(())
}

struct BroadcastState<T> {
    receiver: broadcast::Receiver<T>,
    completion: watch::Receiver<Option<ZaiResult<()>>>,
    channel_name: &'static str,
    terminal_reported: bool,
    completion_lost: bool,
}

fn subscribe_with_initial_backlog<T: Clone>(
    sender: &broadcast::Sender<T>,
    initial: &Mutex<Option<broadcast::Receiver<T>>>,
) -> broadcast::Receiver<T> {
    initial
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .take()
        .unwrap_or_else(|| sender.subscribe())
}

fn observable_broadcast_stream<T>(
    receiver: broadcast::Receiver<T>,
    completion: watch::Receiver<Option<ZaiResult<()>>>,
    channel_name: &'static str,
) -> Pin<Box<dyn Stream<Item = ZaiResult<T>> + Send>>
where
    T: Clone + Send + 'static,
{
    let state = BroadcastState {
        receiver,
        completion,
        channel_name,
        terminal_reported: false,
        completion_lost: false,
    };
    Box::pin(stream::unfold(state, |mut state| async move {
        loop {
            if state.terminal_reported {
                return None;
            }

            if state.completion_lost {
                match state.receiver.try_recv() {
                    Ok(value) => return Some((Ok(value), state)),
                    Err(broadcast::error::TryRecvError::Lagged(skipped)) => {
                        let error = lagged_stream_error(state.channel_name, skipped);
                        state.terminal_reported = true;
                        return Some((Err(error), state));
                    },
                    Err(
                        broadcast::error::TryRecvError::Empty
                        | broadcast::error::TryRecvError::Closed,
                    ) => {
                        state.terminal_reported = true;
                        return Some((
                            Err(protocol_error(
                                "realtime background task ended without a completion status",
                            )),
                            state,
                        ));
                    },
                }
            }

            let completion = state.completion.borrow().clone();
            if let Some(result) = completion {
                match state.receiver.try_recv() {
                    Ok(value) => return Some((Ok(value), state)),
                    Err(broadcast::error::TryRecvError::Lagged(skipped)) => {
                        let error = lagged_stream_error(state.channel_name, skipped);
                        state.terminal_reported = true;
                        return Some((Err(error), state));
                    },
                    Err(
                        broadcast::error::TryRecvError::Empty
                        | broadcast::error::TryRecvError::Closed,
                    ) => match result {
                        Ok(()) => return None,
                        Err(error) => {
                            state.terminal_reported = true;
                            return Some((Err(error), state));
                        },
                    },
                }
            }

            tokio::select! {
                value = state.receiver.recv() => match value {
                    Ok(value) => return Some((Ok(value), state)),
                    Err(broadcast::error::RecvError::Lagged(skipped)) => {
                        let error = lagged_stream_error(state.channel_name, skipped);
                        state.terminal_reported = true;
                        return Some((Err(error), state));
                    },
                    Err(broadcast::error::RecvError::Closed) => return None,
                },
                changed = state.completion.changed() => {
                    if changed.is_err() {
                        state.completion_lost = true;
                    }
                },
            }
        }
    }))
}

fn lagged_stream_error(channel_name: &str, skipped: u64) -> crate::ZaiError {
    protocol_error(format!(
        "{channel_name} consumer lagged and lost {skipped} message(s)"
    ))
}

fn serialize_event(event: &ClientEvent) -> ZaiResult<String> {
    let json = serde_json::to_string(event)?;
    if json.len() as u64 > WS_MESSAGE_MAX {
        return Err(protocol_error(format!(
            "realtime message exceeds {WS_MESSAGE_MAX} bytes"
        )));
    }
    Ok(json)
}

const AUDIO_APPEND_PREFIX: &[u8] = b"{\"type\":\"input_audio_buffer.append\",\"audio\":\"";
const VIDEO_FRAME_APPEND_PREFIX: &[u8] =
    b"{\"type\":\"input_audio_buffer.append_video_frame\",\"video_frame\":\"";
const CLIENT_TIMESTAMP_PREFIX: &[u8] = b"\",\"client_timestamp\":";
const MEDIA_EVENT_WITHOUT_TIMESTAMP_END: &[u8] = b"\"}";

/// Serialize an outbound audio append without materializing a second base64
/// `String`. The caller holds the preparation permit while this runs, and the
/// returned final wire buffer is subsequently charged to the existing exact
/// byte/slot admission budgets.
fn serialize_audio_append(
    pcm: &[u8],
    format: InputAudioFormat,
    client_timestamp: Option<i64>,
) -> ZaiResult<String> {
    match format {
        InputAudioFormat::Wav => {
            let header = wav_pcm_header(pcm.len(), 16_000)?;
            serialize_base64_media_event(AUDIO_APPEND_PREFIX, [&header, pcm], client_timestamp)
        },
        InputAudioFormat::Pcm16 | InputAudioFormat::Pcm24 => {
            serialize_base64_media_event(AUDIO_APPEND_PREFIX, [pcm], client_timestamp)
        },
    }
}

fn serialize_video_frame_append(jpeg: &[u8], client_timestamp: Option<i64>) -> ZaiResult<String> {
    serialize_base64_media_event(VIDEO_FRAME_APPEND_PREFIX, [jpeg], client_timestamp)
}

fn serialize_base64_media_event<const N: usize>(
    prefix: &[u8],
    chunks: [&[u8]; N],
    client_timestamp: Option<i64>,
) -> ZaiResult<String> {
    let decoded_len = chunks.iter().try_fold(0usize, |total, chunk| {
        total
            .checked_add(chunk.len())
            .ok_or_else(|| protocol_error("realtime media payload length overflow"))
    })?;
    let encoded_len = base64::encoded_len(decoded_len, true)
        .ok_or_else(|| protocol_error("realtime media base64 length overflow"))?;
    let final_len = media_event_wire_len(prefix.len(), encoded_len, client_timestamp)?;

    // This is the sole payload-sized allocation. EncoderWriter owns only its
    // fixed stack buffer and writes base64 directly behind the JSON prefix.
    let mut json = Vec::with_capacity(final_len);
    json.extend_from_slice(prefix);
    {
        let mut encoder = base64::write::EncoderWriter::new(
            &mut json,
            &base64::engine::general_purpose::STANDARD,
        );
        for chunk in chunks {
            encoder.write_all(chunk).map_err(|error| {
                protocol_error(format!("realtime media base64 encode failed: {error}"))
            })?;
        }
        encoder.finish().map_err(|error| {
            protocol_error(format!("realtime media base64 encode failed: {error}"))
        })?;
    }

    match client_timestamp {
        Some(timestamp) => {
            json.extend_from_slice(CLIENT_TIMESTAMP_PREFIX);
            serde_json::to_writer(&mut json, &timestamp)?;
            json.push(b'}');
        },
        None => json.extend_from_slice(MEDIA_EVENT_WITHOUT_TIMESTAMP_END),
    }
    debug_assert_eq!(json.len(), final_len);

    String::from_utf8(json)
        .map_err(|_| protocol_error("realtime media base64 encoder produced invalid UTF-8"))
}

fn media_event_wire_len(
    prefix_len: usize,
    encoded_len: usize,
    client_timestamp: Option<i64>,
) -> ZaiResult<usize> {
    let suffix_len = match client_timestamp {
        Some(timestamp) => CLIENT_TIMESTAMP_PREFIX
            .len()
            .checked_add(decimal_i64_len(timestamp))
            .and_then(|len| len.checked_add(1)),
        None => Some(MEDIA_EVENT_WITHOUT_TIMESTAMP_END.len()),
    }
    .ok_or_else(|| protocol_error("realtime media JSON length overflow"))?;
    let final_len = prefix_len
        .checked_add(encoded_len)
        .and_then(|len| len.checked_add(suffix_len))
        .ok_or_else(|| protocol_error("realtime media JSON length overflow"))?;
    if u64::try_from(final_len).unwrap_or(u64::MAX) > WS_MESSAGE_MAX {
        return Err(protocol_error(format!(
            "realtime message exceeds {WS_MESSAGE_MAX} bytes"
        )));
    }
    Ok(final_len)
}

fn decimal_i64_len(value: i64) -> usize {
    usize::from(value.is_negative())
        + if value == 0 {
            1
        } else {
            value.unsigned_abs().ilog10() as usize + 1
        }
}

fn wav_pcm_header(samples_len: usize, sample_rate: u32) -> ZaiResult<[u8; 44]> {
    if !samples_len.is_multiple_of(2) {
        return Err(protocol_error(
            "16-bit PCM input must contain an even number of bytes",
        ));
    }
    if sample_rate == 0 {
        return Err(protocol_error("WAV sample rate must be positive"));
    }

    const CHANNELS: u16 = 1;
    const BYTES_PER_SAMPLE: u16 = 2;
    let block_align = CHANNELS
        .checked_mul(BYTES_PER_SAMPLE)
        .ok_or_else(|| protocol_error("WAV block alignment overflow"))?;
    let byte_rate = sample_rate
        .checked_mul(u32::from(block_align))
        .ok_or_else(|| protocol_error("WAV byte rate overflow"))?;
    let data_len =
        u32::try_from(samples_len).map_err(|_| protocol_error("PCM input is too large for WAV"))?;
    let chunk_size = data_len
        .checked_add(36)
        .ok_or_else(|| protocol_error("PCM input is too large for WAV"))?;

    let mut header = [0u8; 44];
    header[0..4].copy_from_slice(b"RIFF");
    header[4..8].copy_from_slice(&chunk_size.to_le_bytes());
    header[8..12].copy_from_slice(b"WAVE");
    header[12..16].copy_from_slice(b"fmt ");
    header[16..20].copy_from_slice(&16u32.to_le_bytes());
    header[20..22].copy_from_slice(&1u16.to_le_bytes());
    header[22..24].copy_from_slice(&CHANNELS.to_le_bytes());
    header[24..28].copy_from_slice(&sample_rate.to_le_bytes());
    header[28..32].copy_from_slice(&byte_rate.to_le_bytes());
    header[32..34].copy_from_slice(&block_align.to_le_bytes());
    header[34..36].copy_from_slice(&(BYTES_PER_SAMPLE * 8).to_le_bytes());
    header[36..40].copy_from_slice(b"data");
    header[40..44].copy_from_slice(&data_len.to_le_bytes());
    Ok(header)
}

#[cfg(test)]
#[global_allocator]
static MEDIA_SERIALIZER_TEST_ALLOCATOR: &stats_alloc::StatsAlloc<std::alloc::System> =
    &stats_alloc::INSTRUMENTED_SYSTEM;

#[cfg(test)]
mod media_serializer_tests {
    use std::hint::black_box;

    use stats_alloc::Region;

    use super::*;

    fn audio_serde_oracle(
        pcm: &[u8],
        format: InputAudioFormat,
        client_timestamp: Option<i64>,
    ) -> String {
        let audio = match format {
            InputAudioFormat::Wav => {
                super::super::audio::encode_wav_pcm_base64(pcm, 16_000).unwrap()
            },
            InputAudioFormat::Pcm16 | InputAudioFormat::Pcm24 => {
                super::super::audio::encode_base64(pcm)
            },
        };
        serde_json::to_string(&ClientEvent::InputAudioBufferAppend {
            audio,
            client_timestamp,
        })
        .unwrap()
    }

    fn video_serde_oracle(jpeg: &[u8], client_timestamp: Option<i64>) -> String {
        serde_json::to_string(&ClientEvent::InputAudioBufferAppendVideoFrame {
            video_frame: super::super::audio::encode_jpeg_frame_base64(jpeg),
            client_timestamp,
        })
        .unwrap()
    }

    #[test]
    fn private_media_serializer_is_byte_identical_to_public_event_serde() {
        let timestamps = [
            None,
            Some(0),
            Some(-1),
            Some(1_731_999_464_667),
            Some(i64::MIN),
            Some(i64::MAX),
        ];

        for format in [
            InputAudioFormat::Wav,
            InputAudioFormat::Pcm16,
            InputAudioFormat::Pcm24,
        ] {
            for size in [0, 2, 4, 640] {
                let pcm = (0..size).map(|index| index as u8).collect::<Vec<_>>();
                for timestamp in timestamps {
                    let expected = audio_serde_oracle(&pcm, format, timestamp);
                    let actual = serialize_audio_append(&pcm, format, timestamp).unwrap();
                    assert_eq!(actual.as_bytes(), expected.as_bytes());
                    assert_eq!(actual.len(), actual.capacity());
                }
            }
        }

        for size in 0..=6 {
            let jpeg = (0..size).map(|index| index as u8).collect::<Vec<_>>();
            for timestamp in timestamps {
                let expected = video_serde_oracle(&jpeg, timestamp);
                let actual = serialize_video_frame_append(&jpeg, timestamp).unwrap();
                assert_eq!(actual.as_bytes(), expected.as_bytes());
                assert_eq!(actual.len(), actual.capacity());
            }
        }
    }

    #[test]
    fn private_media_serializer_checks_metadata_and_wire_boundaries() {
        assert!(wav_pcm_header(1, 16_000).is_err());
        assert!(wav_pcm_header(2, 0).is_err());
        assert_eq!(wav_pcm_header(0, 16_000).unwrap()[40..44], [0; 4]);

        for timestamp in [0, -1, i64::MIN, i64::MAX] {
            assert_eq!(
                decimal_i64_len(timestamp),
                serde_json::to_string(&timestamp).unwrap().len()
            );
        }

        let timestamp = Some(i64::MIN);
        let suffix_len = CLIENT_TIMESTAMP_PREFIX.len() + decimal_i64_len(i64::MIN) + 1;
        let max_encoded_len = WS_MESSAGE_MAX as usize - AUDIO_APPEND_PREFIX.len() - suffix_len;
        assert_eq!(
            media_event_wire_len(AUDIO_APPEND_PREFIX.len(), max_encoded_len, timestamp).unwrap(),
            WS_MESSAGE_MAX as usize
        );
        let error = media_event_wire_len(AUDIO_APPEND_PREFIX.len(), max_encoded_len + 1, timestamp)
            .expect_err("media JSON larger than the WebSocket cap was accepted");
        assert!(error.message().contains("realtime message exceeds"));
    }

    #[test]
    fn realtime_media_serializer_allocation_gate() {
        const CHILD_ENV: &str = "ZAI_REALTIME_MEDIA_ALLOC_CHILD";
        const EXACT_TEST_NAME: &str = concat!(
            "realtime::session::media_serializer_tests::",
            "realtime_media_serializer_allocation_gate"
        );

        // stats_alloc counters are process-global. Keep this as an ordinary
        // all-targets gate while isolating the actual census in a child that
        // runs only this test on one thread.
        if std::env::var_os(CHILD_ENV).is_none() {
            let status = std::process::Command::new(std::env::current_exe().unwrap())
                .args([EXACT_TEST_NAME, "--exact", "--test-threads=1"])
                .env(CHILD_ENV, "1")
                .status()
                .unwrap();
            assert!(status.success(), "isolated allocation census failed");
            return;
        }

        const PAYLOAD_SIZES: [usize; 3] = [640, 64 * 1024, 4 * 1024 * 1024];
        const MAX_ALLOCATIONS: usize = 2;
        const MAX_FIXED_OVERHEAD: usize = 64;
        const TIMESTAMP: Option<i64> = Some(1_731_999_464_667);

        for payload_size in PAYLOAD_SIZES {
            let payload = vec![0x5a; payload_size];
            for format in [
                InputAudioFormat::Wav,
                InputAudioFormat::Pcm16,
                InputAudioFormat::Pcm24,
            ] {
                // Build the public-Serde oracle before opening the measured
                // region; only the private serializer is counted below.
                let oracle = audio_serde_oracle(&payload, format, TIMESTAMP);
                let region = Region::new(MEDIA_SERIALIZER_TEST_ALLOCATOR);
                let actual = black_box(
                    serialize_audio_append(black_box(&payload), format, TIMESTAMP).unwrap(),
                );
                let stats = region.change();

                assert_eq!(actual.as_bytes(), oracle.as_bytes());
                assert!(
                    stats.allocations <= MAX_ALLOCATIONS,
                    "{format:?} {payload_size}-byte payload allocated too often: {stats:?}"
                );
                assert_eq!(
                    stats.reallocations, 0,
                    "{format:?} {payload_size}-byte payload reallocated: {stats:?}"
                );
                assert!(
                    stats.bytes_allocated <= actual.len() + MAX_FIXED_OVERHEAD,
                    "{format:?} {payload_size}-byte payload duplicated storage: {stats:?}"
                );
            }

            let oracle = video_serde_oracle(&payload, TIMESTAMP);
            let region = Region::new(MEDIA_SERIALIZER_TEST_ALLOCATOR);
            let actual =
                black_box(serialize_video_frame_append(black_box(&payload), TIMESTAMP).unwrap());
            let stats = region.change();

            assert_eq!(actual.as_bytes(), oracle.as_bytes());
            assert!(
                stats.allocations <= MAX_ALLOCATIONS,
                "JPEG {payload_size}-byte payload allocated too often: {stats:?}"
            );
            assert_eq!(
                stats.reallocations, 0,
                "JPEG {payload_size}-byte payload reallocated: {stats:?}"
            );
            assert!(
                stats.bytes_allocated <= actual.len() + MAX_FIXED_OVERHEAD,
                "JPEG {payload_size}-byte payload duplicated storage: {stats:?}"
            );
        }
    }
}

fn prepare_session_update(
    model_name: &str,
    mut session_config: SessionConfig,
) -> ZaiResult<(String, InputAudioFormat)> {
    // The selected type-safe model is part of the session.update wire
    // contract. It takes precedence over an arbitrary value supplied via
    // `session_config` so the marker-trait guarantee cannot be bypassed.
    session_config.model = Some(model_name.to_owned());
    validate_session_config(&session_config)?;
    let input_audio_format = session_config.input_audio_format;
    let init = ClientEvent::SessionUpdate {
        event_id: Some(new_event_id()),
        session: session_config,
    };
    // Serialize and enforce the message limit before opening a socket so a
    // locally invalid configuration cannot cause network side effects.
    Ok((serialize_event(&init)?, input_audio_format))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ConnectRetry {
    retry_after: Option<Duration>,
}

struct RealtimeSystemJitter;

impl JitterSource for RealtimeSystemJitter {
    fn jitter(&self, upper: Duration) -> Duration {
        let upper_nanos = u64::try_from(upper.as_nanos()).unwrap_or(u64::MAX);
        if upper_nanos == 0 {
            return Duration::ZERO;
        }
        Duration::from_nanos(fastrand::u64(0..=upper_nanos))
    }
}

async fn connect_before_session_update(
    api_key: &ApiSecret,
    auth: AuthMode,
    realtime_url: &str,
    transport_config: &RealtimeTransportConfig,
) -> ZaiResult<super::transport::TungsteniteTransport> {
    connect_before_session_update_with(
        transport_config.connect_timeout(),
        transport_config.max_connect_attempts(),
        &RealtimeSystemJitter,
        || jwt::authorization_header(api_key.expose(), jwt_ttl(auth)),
        |authorization| {
            let transport_config = transport_config.clone();
            async move {
                super::transport::TungsteniteTransport::connect_with_config(
                    realtime_url,
                    &authorization,
                    transport_config,
                )
                .await
            }
        },
    )
    .await
}

/// Retry only the built-in connection handshake, before any application frame
/// can be sent. `connect_timeout` is one absolute budget shared by attempts and
/// backoff. The separate authorization factory is deliberately invoked for
/// every attempt so a short-lived JWT cannot expire while retrying.
async fn connect_before_session_update_with<T, A, C, Fut>(
    connect_timeout: Duration,
    max_attempts: u8,
    jitter: &dyn JitterSource,
    mut authorization: A,
    mut connector: C,
) -> ZaiResult<T>
where
    A: FnMut() -> ZaiResult<String>,
    C: FnMut(String) -> Fut,
    Fut: Future<Output = ZaiResult<T>>,
{
    debug_assert!((1..=RealtimeTransportConfig::MAX_CONNECT_ATTEMPTS).contains(&max_attempts));
    let deadline = tokio::time::Instant::now() + connect_timeout;
    let mut attempt = 1_u8;

    loop {
        if tokio::time::Instant::now() >= deadline {
            return Err(connect_timeout_error());
        }

        let authorization = authorization()?;
        if tokio::time::Instant::now() >= deadline {
            return Err(connect_timeout_error());
        }
        let result = tokio::time::timeout_at(deadline, connector(authorization))
            .await
            .map_err(|_| connect_timeout_error())?;
        let error = match result {
            Ok(transport) => return Ok(transport),
            Err(error) => error,
        };

        let Some(retry) = classify_connect_failure(&error) else {
            return Err(error);
        };
        if attempt >= max_attempts {
            return Err(error);
        }

        let computed = backoff_delay(u32::from(attempt) - 1, jitter);
        let delay = reconcile_retry_after(retry.retry_after, computed);
        let now = tokio::time::Instant::now();
        let Some(retry_at) = now.checked_add(delay) else {
            return Err(error);
        };
        // Preserve the concrete handshake failure when its required backoff
        // cannot leave any time for another attempt. A timeout is returned only
        // when the absolute budget itself actually expires.
        if retry_at >= deadline {
            return Err(error);
        }

        let delay_ms = u64::try_from(delay.as_millis()).unwrap_or(u64::MAX);
        debug!(
            failed_attempt = attempt,
            next_attempt = attempt + 1,
            max_attempts,
            delay_ms,
            "Retrying realtime connection before session.update"
        );
        tokio::time::sleep_until(retry_at).await;
        if tokio::time::Instant::now() >= deadline {
            return Err(connect_timeout_error());
        }
        attempt += 1;
    }
}

fn classify_connect_failure(error: &ZaiError) -> Option<ConnectRetry> {
    let ZaiError::RealtimeError(kind) = error.source_error() else {
        return None;
    };

    match kind.as_ref() {
        RealtimeErrorKind::HandshakeHttp(context) if error.is_retryable() => Some(ConnectRetry {
            retry_after: context.retry_after(),
        }),
        RealtimeErrorKind::WebSocket {
            source: WebSocketError::Io(_),
        } if error.is_retryable() => Some(ConnectRetry { retry_after: None }),
        // The public error projection owns the HTTP/business and I/O allowlists.
        // Other WebSocket, protocol, TLS, URL, and outer-timeout shapes fail
        // closed here even if a future caller-facing category becomes broader.
        _ => None,
    }
}

fn connect_timeout_error() -> ZaiError {
    RealtimeErrorKind::Timeout {
        operation: "WebSocket connect",
    }
    .into()
}

const fn jwt_ttl(auth: AuthMode) -> Option<i64> {
    match auth {
        AuthMode::Bearer => None,
        AuthMode::Jwt { ttl_seconds } => Some(ttl_seconds),
    }
}

fn protocol_error(message: impl Into<String>) -> crate::ZaiError {
    RealtimeErrorKind::Protocol(message.into()).into()
}

fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

fn new_event_id() -> String {
    format!("evt_{}", uuid::Uuid::new_v4().simple())
}

#[cfg(test)]
mod connect_retry_tests {
    use super::*;
    use futures_util::future::BoxFuture;
    use std::{collections::VecDeque, future, io::ErrorKind, sync::atomic::AtomicUsize};
    use tokio::sync::Notify;
    use tokio_tungstenite::tungstenite::error::{ProtocolError, TlsError, UrlError};

    struct FixedJitter(Duration);

    impl JitterSource for FixedJitter {
        fn jitter(&self, upper: Duration) -> Duration {
            self.0.min(upper)
        }
    }

    fn io_connect_error(kind: ErrorKind) -> ZaiError {
        WebSocketError::Io(std::io::Error::new(kind, "scripted connect failure")).into()
    }

    fn http_connect_error(status: u16, body: &[u8], retry_after: Option<&str>) -> ZaiError {
        let mut response = http::Response::builder()
            .status(status)
            .header(http::header::CONTENT_LENGTH, body.len().to_string());
        if let Some(retry_after) = retry_after {
            response = response.header(http::header::RETRY_AFTER, retry_after);
        }
        WebSocketError::Http(Box::new(response.body(Some(body.to_vec())).unwrap())).into()
    }

    #[test]
    fn connect_classifier_matches_the_public_retry_projection_for_safe_shapes() {
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
            let error = io_connect_error(kind);
            assert!(error.is_retryable(), "{kind:?} should be retryable");
            assert_eq!(
                classify_connect_failure(&error),
                Some(ConnectRetry { retry_after: None })
            );
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
            let error = io_connect_error(kind);
            assert!(!error.is_retryable(), "{kind:?} must fail closed");
            assert_eq!(classify_connect_failure(&error), None);
        }

        let protocol: ZaiError =
            WebSocketError::Protocol(ProtocolError::HandshakeIncomplete).into();
        assert!(!protocol.is_retryable());
        assert!(classify_connect_failure(&protocol).is_none());
        let url: ZaiError = WebSocketError::Url(UrlError::UnsupportedUrlScheme).into();
        assert!(!url.is_retryable());
        assert!(classify_connect_failure(&url).is_none());
        let tls: ZaiError = WebSocketError::Tls(TlsError::InvalidDnsName).into();
        assert!(!tls.is_retryable());
        assert!(classify_connect_failure(&tls).is_none());
        let timeout = connect_timeout_error();
        assert!(timeout.is_retryable());
        assert!(classify_connect_failure(&timeout).is_none());
    }

    #[test]
    fn handshake_http_retry_uses_business_code_and_retry_after() {
        let transient = http_connect_error(503, br#"{"code":1302}"#, Some("2"));
        assert!(transient.is_retryable());
        assert!(transient.is_rate_limit());
        assert_eq!(
            classify_connect_failure(&transient),
            Some(ConnectRetry {
                retry_after: Some(Duration::from_secs(2))
            })
        );

        let quota = http_connect_error(429, br#"{"error":{"code":1113}}"#, Some("1"));
        assert!(quota.is_rate_limit());
        assert!(!quota.is_retryable());
        assert_eq!(classify_connect_failure(&quota), None);

        let unframed_tail: ZaiError = WebSocketError::Http(Box::new(
            http::Response::builder()
                .status(400)
                .body(Some(br#"{"code":1302}"#.to_vec()))
                .unwrap(),
        ))
        .into();
        assert_eq!(
            unframed_tail.category(),
            crate::client::error::ErrorCategory::Client
        );
        assert!(!unframed_tail.is_retryable());
        assert_eq!(classify_connect_failure(&unframed_tail), None);

        let auth = http_connect_error(401, br#"{"message":"bad key"}"#, None);
        assert!(auth.is_auth_error());
        assert!(!auth.is_retryable());
        assert_eq!(classify_connect_failure(&auth), None);
        let malformed_upgrade: ZaiError =
            WebSocketError::Protocol(ProtocolError::MissingUpgradeWebSocketHeader).into();
        assert!(!malformed_upgrade.is_retryable());
        assert_eq!(classify_connect_failure(&malformed_upgrade), None);
    }

    #[tokio::test]
    async fn retries_refresh_authorization_and_stop_after_success() {
        let mut authorizations = 0_usize;
        let mut connections = 0_usize;
        let mut outcomes = VecDeque::from([
            Err(io_connect_error(ErrorKind::ConnectionRefused)),
            Ok(7_u8),
            Err(io_connect_error(ErrorKind::ConnectionReset)),
        ]);

        let value = connect_before_session_update_with(
            Duration::from_secs(2),
            3,
            &FixedJitter(Duration::ZERO),
            || {
                authorizations += 1;
                Ok(format!("Bearer attempt-{authorizations}"))
            },
            |authorization| {
                connections += 1;
                assert_eq!(authorization, format!("Bearer attempt-{connections}"));
                future::ready(outcomes.pop_front().unwrap())
            },
        )
        .await
        .unwrap();

        assert_eq!(value, 7);
        assert_eq!(authorizations, 2);
        assert_eq!(connections, 2);
        assert_eq!(
            outcomes.len(),
            1,
            "success must stop before another attempt"
        );
    }

    #[tokio::test]
    async fn connect_attempt_cap_and_single_attempt_override_are_exact() {
        for (max_attempts, expected) in [(1_u8, 1_usize), (3, 3)] {
            let mut authorizations = 0_usize;
            let mut connections = 0_usize;
            let error = connect_before_session_update_with::<(), _, _, _>(
                Duration::from_secs(2),
                max_attempts,
                &FixedJitter(Duration::ZERO),
                || {
                    authorizations += 1;
                    Ok("Bearer test".to_string())
                },
                |_| {
                    connections += 1;
                    future::ready(Err(http_connect_error(503, b"{}", None)))
                },
            )
            .await
            .unwrap_err();

            assert!(matches!(
                error,
                ZaiError::RealtimeError(ref kind)
                    if matches!(kind.as_ref(), RealtimeErrorKind::HandshakeHttp(_))
            ));
            assert!(error.is_retryable());
            assert_eq!(authorizations, expected);
            assert_eq!(connections, expected);
        }
    }

    #[tokio::test(start_paused = true)]
    async fn handshake_and_backoff_share_one_absolute_connect_budget() {
        let started = tokio::time::Instant::now();
        let connections = Arc::new(AtomicUsize::new(0));
        let observed = Arc::clone(&connections);
        let error = connect_before_session_update_with(
            Duration::from_secs(2),
            3,
            &FixedJitter(Duration::ZERO),
            || Ok("Bearer test".to_string()),
            move |_| -> BoxFuture<'static, ZaiResult<()>> {
                let attempt = observed.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                if attempt == 0 {
                    Box::pin(async { Err(http_connect_error(503, b"{}", Some("1"))) })
                } else {
                    Box::pin(std::future::pending())
                }
            },
        )
        .await
        .unwrap_err();

        assert!(matches!(
            error,
            ZaiError::RealtimeError(ref kind)
                if matches!(kind.as_ref(), RealtimeErrorKind::Timeout { operation: "WebSocket connect" })
        ));
        assert_eq!(
            tokio::time::Instant::now() - started,
            Duration::from_secs(2)
        );
        assert_eq!(connections.load(std::sync::atomic::Ordering::SeqCst), 2);
    }

    #[tokio::test(start_paused = true)]
    async fn retry_after_that_consumes_budget_preserves_handshake_error() {
        let started = tokio::time::Instant::now();
        let mut connections = 0_usize;
        let error = connect_before_session_update_with::<(), _, _, _>(
            Duration::from_secs(1),
            3,
            &FixedJitter(Duration::ZERO),
            || Ok("Bearer test".to_string()),
            |_| {
                connections += 1;
                future::ready(Err(http_connect_error(503, b"{}", Some("1"))))
            },
        )
        .await
        .unwrap_err();

        assert!(matches!(
            error,
            ZaiError::RealtimeError(ref kind)
                if matches!(kind.as_ref(), RealtimeErrorKind::HandshakeHttp(_))
        ));
        assert!(error.is_retryable());
        assert_eq!(tokio::time::Instant::now(), started);
        assert_eq!(connections, 1);
    }

    #[tokio::test(start_paused = true)]
    async fn cancelling_backoff_does_not_start_another_attempt() {
        let attempts = Arc::new(AtomicUsize::new(0));
        let first_attempt = Arc::new(Notify::new());
        let task_attempts = Arc::clone(&attempts);
        let task_first_attempt = Arc::clone(&first_attempt);
        let task = tokio::spawn(async move {
            connect_before_session_update_with::<(), _, _, _>(
                Duration::from_secs(10),
                3,
                &FixedJitter(Duration::from_secs(5)),
                || Ok("Bearer test".to_string()),
                move |_| {
                    task_attempts.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    task_first_attempt.notify_one();
                    future::ready(Err(io_connect_error(ErrorKind::ConnectionRefused)))
                },
            )
            .await
        });

        first_attempt.notified().await;
        tokio::task::yield_now().await;
        task.abort();
        assert!(task.await.unwrap_err().is_cancelled());
        tokio::time::advance(Duration::from_secs(10)).await;
        assert_eq!(attempts.load(std::sync::atomic::Ordering::SeqCst), 1);
    }
}

#[cfg(test)]
mod teardown_tests {
    use super::*;
    use async_trait::async_trait;
    use std::sync::Mutex;

    /// A mock transport whose `recv` never resolves, so the only way the event
    /// loop can exit is via the command branch — pinning the drop→teardown
    /// invariant `RealtimeSession::close` depends on.
    struct HangingTransport {
        closed: Arc<Mutex<bool>>,
    }

    #[async_trait]
    impl RealtimeTransport for HangingTransport {
        async fn send(&mut self, _msg: String) -> crate::ZaiResult<()> {
            Ok(())
        }
        async fn recv(&mut self) -> crate::ZaiResult<Option<WsMessage>> {
            // Never resolves: forces the loop to exit via the command channel.
            std::future::pending().await
        }
        async fn close(&mut self) -> crate::ZaiResult<()> {
            *self.closed.lock().unwrap() = true;
            Ok(())
        }
    }

    /// Regression guard: dropping the last command `Sender` must terminate the
    /// background loop AND close the transport. The consuming `close()` and the
    /// implicit-drop teardown both rely on this; a future refactor that drops
    /// the `None => close` arm would leak the task. Uses a 2s timeout so a
    /// regression fails fast instead of hanging the test binary.
    #[tokio::test]
    async fn dropping_command_sender_terminates_loop_and_closes_transport() {
        let closed = Arc::new(Mutex::new(false));
        let transport = HangingTransport {
            closed: Arc::clone(&closed),
        };
        let (cmd_tx, cmd_rx) = mpsc::channel::<String>(8);
        let (_shutdown_tx, shutdown_rx) = watch::channel(false);
        let (events_tx, _) = broadcast::channel::<ServerEvent>(16);
        let (audio_tx, _) = broadcast::channel::<RealtimeAudioChunk>(16);
        let join = tokio::spawn(run_loop(
            transport,
            cmd_rx,
            shutdown_rx,
            events_tx,
            audio_tx,
            RealtimeTransportConfig::default(),
        ));

        // Drop the last sender → `cmd_rx.recv()` returns `None` → the loop
        // calls `transport.close()` and exits.
        drop(cmd_tx);

        let joined = tokio::time::timeout(std::time::Duration::from_secs(2), join)
            .await
            .expect("run_loop did not terminate after the command sender dropped");
        joined
            .expect("run_loop task panicked")
            .expect("run_loop returned an error");
        assert!(
            *closed.lock().unwrap(),
            "transport.close() was not invoked on teardown"
        );
    }
}

#[cfg(test)]
mod run_loop_tests {
    use super::*;
    use async_trait::async_trait;
    use base64::Engine as _;
    use futures_util::StreamExt as _;
    use std::collections::VecDeque;
    use std::sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    };
    use std::time::Duration;
    use tokio::sync::Notify;

    /// Mock transport with scripted responses.
    struct ScriptedTransport {
        messages: VecDeque<String>,
        disconnect_when_empty: bool,
        sent: Arc<Mutex<Vec<String>>>,
        closed: Arc<Mutex<bool>>,
    }

    impl ScriptedTransport {
        fn new(msgs: Vec<&str>) -> Self {
            Self {
                messages: msgs.into_iter().map(String::from).collect(),
                disconnect_when_empty: false,
                sent: Arc::new(Mutex::new(Vec::new())),
                closed: Arc::new(Mutex::new(false)),
            }
        }

        fn disconnecting() -> Self {
            Self {
                disconnect_when_empty: true,
                ..Self::new(Vec::new())
            }
        }
    }

    #[async_trait]
    impl RealtimeTransport for ScriptedTransport {
        async fn send(&mut self, msg: String) -> crate::ZaiResult<()> {
            self.sent.lock().unwrap().push(msg);
            Ok(())
        }

        async fn recv(&mut self) -> crate::ZaiResult<Option<WsMessage>> {
            match self.messages.pop_front() {
                Some(message) => Ok(Some(WsMessage::Text(message))),
                None if self.disconnect_when_empty => Ok(None),
                None => std::future::pending().await,
            }
        }
        async fn close(&mut self) -> crate::ZaiResult<()> {
            *self.closed.lock().unwrap() = true;
            Ok(())
        }
    }

    struct FloodTransport {
        received: Arc<AtomicUsize>,
        sent: Arc<AtomicUsize>,
        closed: Arc<AtomicBool>,
    }

    #[async_trait]
    impl RealtimeTransport for FloodTransport {
        async fn send(&mut self, _msg: String) -> crate::ZaiResult<()> {
            self.sent.fetch_add(1, Ordering::Relaxed);
            Ok(())
        }

        async fn recv(&mut self) -> crate::ZaiResult<Option<WsMessage>> {
            self.received.fetch_add(1, Ordering::Relaxed);
            Ok(Some(WsMessage::Text(r#"{"type":"heartbeat"}"#.into())))
        }

        async fn close(&mut self) -> crate::ZaiResult<()> {
            self.closed.store(true, Ordering::Relaxed);
            Ok(())
        }
    }

    struct BoundaryHeartbeatTransport {
        delivered: bool,
        closed: Arc<AtomicBool>,
    }

    #[async_trait]
    impl RealtimeTransport for BoundaryHeartbeatTransport {
        async fn send(&mut self, _msg: String) -> crate::ZaiResult<()> {
            Ok(())
        }

        async fn recv(&mut self) -> crate::ZaiResult<Option<WsMessage>> {
            if self.delivered {
                return std::future::pending().await;
            }
            self.delivered = true;
            tokio::time::sleep(RealtimeTransportConfig::default().inbound_idle_timeout()).await;
            Ok(Some(WsMessage::Text(r#"{"type":"heartbeat"}"#.into())))
        }

        async fn close(&mut self) -> crate::ZaiResult<()> {
            self.closed.store(true, Ordering::Relaxed);
            Ok(())
        }
    }

    struct BlockingSendTransport {
        send_started: Arc<Notify>,
        closed: Arc<AtomicBool>,
    }

    #[async_trait]
    impl RealtimeTransport for BlockingSendTransport {
        async fn send(&mut self, _msg: String) -> crate::ZaiResult<()> {
            self.send_started.notify_one();
            std::future::pending().await
        }

        async fn recv(&mut self) -> crate::ZaiResult<Option<WsMessage>> {
            std::future::pending().await
        }

        async fn close(&mut self) -> crate::ZaiResult<()> {
            self.closed.store(true, Ordering::Relaxed);
            Ok(())
        }
    }

    /// Mirrors the built-in transport: `send` admits into a bounded writer
    /// queue while an independent wire task can be blocked in the actual sink.
    struct BufferedBlockingWireTransport {
        writer_tx: mpsc::Sender<String>,
        heartbeat_delivered: bool,
        writer_shutdown: Arc<Notify>,
        closed: Arc<AtomicBool>,
    }

    #[async_trait]
    impl RealtimeTransport for BufferedBlockingWireTransport {
        async fn send(&mut self, msg: String) -> crate::ZaiResult<()> {
            self.writer_tx
                .try_send(msg)
                .map_err(|_| protocol_error("test writer queue unavailable"))
        }

        async fn recv(&mut self) -> crate::ZaiResult<Option<WsMessage>> {
            if self.heartbeat_delivered {
                return std::future::pending().await;
            }
            tokio::time::sleep(Duration::from_secs(30)).await;
            // Mutate only after the cancellation-safe wait completes: the
            // session select may drop this future when an outbound command is
            // admitted to the independent writer.
            self.heartbeat_delivered = true;
            Ok(Some(WsMessage::Text(r#"{"type":"heartbeat"}"#.into())))
        }

        async fn close(&mut self) -> crate::ZaiResult<()> {
            self.closed.store(true, Ordering::Relaxed);
            self.writer_shutdown.notify_one();
            Ok(())
        }
    }

    /// A deterministic injected transport used to exercise the public
    /// third-party admission contract independently from a blocked wire task.
    /// `send` admits into its own bounded channel while that task waits on an
    /// explicit release barrier; built-in permit transfer has a separate
    /// private-adapter regression below.
    struct GatedBacklogTransport {
        writer_tx: mpsc::Sender<String>,
        inbound_rx: mpsc::UnboundedReceiver<WsMessage>,
        admitted: Arc<AtomicUsize>,
        admitted_target: usize,
        all_admitted: Arc<Notify>,
        inbound_processed: Arc<AtomicUsize>,
        inbound_target: usize,
        all_inbound_processed: Arc<Notify>,
        writer_shutdown_tx: watch::Sender<bool>,
        writer_join: Option<JoinHandle<()>>,
        close_count: Arc<AtomicUsize>,
    }

    #[async_trait]
    impl RealtimeTransport for GatedBacklogTransport {
        async fn send(&mut self, msg: String) -> crate::ZaiResult<()> {
            match self.writer_tx.try_send(msg) {
                Ok(()) => {},
                Err(mpsc::error::TrySendError::Full(_)) => {
                    return Err(protocol_error("test writer queue is full"));
                },
                Err(mpsc::error::TrySendError::Closed(_)) => {
                    return Err(protocol_error("test writer queue is closed"));
                },
            }
            let admitted = self.admitted.fetch_add(1, Ordering::SeqCst) + 1;
            if admitted == self.admitted_target {
                self.all_admitted.notify_one();
            }
            Ok(())
        }

        async fn recv(&mut self) -> crate::ZaiResult<Option<WsMessage>> {
            let message = self.inbound_rx.recv().await;
            if message.is_some() {
                let processed = self.inbound_processed.fetch_add(1, Ordering::SeqCst) + 1;
                if processed == self.inbound_target {
                    self.all_inbound_processed.notify_one();
                }
            }
            Ok(message)
        }

        async fn close(&mut self) -> crate::ZaiResult<()> {
            self.close_count.fetch_add(1, Ordering::SeqCst);
            let _ = self.writer_shutdown_tx.send(true);
            if let Some(join) = self.writer_join.take() {
                join.await
                    .map_err(|error| protocol_error(format!("test writer join failed: {error}")))?;
            }
            Ok(())
        }
    }

    async fn assert_loop_ok(join: JoinHandle<ZaiResult<()>>) {
        tokio::time::timeout(Duration::from_secs(2), join)
            .await
            .expect("run_loop timed out")
            .expect("run_loop task panicked")
            .expect("run_loop returned an error");
    }

    fn spawn_test_loop<T, C>(
        transport: T,
        cmd_rx: mpsc::Receiver<C>,
        events_tx: broadcast::Sender<ServerEvent>,
        audio_tx: broadcast::Sender<RealtimeAudioChunk>,
    ) -> (watch::Sender<bool>, JoinHandle<ZaiResult<()>>)
    where
        T: RealtimeTransport + 'static,
        C: Into<OutboundCommand> + Send + 'static,
    {
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        let join = tokio::spawn(run_loop(
            transport,
            cmd_rx,
            shutdown_rx,
            events_tx,
            audio_tx,
            RealtimeTransportConfig::default(),
        ));
        (shutdown_tx, join)
    }

    #[tokio::test]
    async fn run_loop_processes_server_events() {
        let transport = ScriptedTransport::new(vec![
            r#"{"type":"session.created","session":{"id":"s1"}}"#,
            r#"{"type":"session.updated","session":{"input_audio_format":"wav","output_audio_format":"pcm","turn_detection":{"type":"client_vad"}}}"#,
        ]);
        let (_cmd_tx, cmd_rx) = mpsc::channel::<String>(8);
        let (events_tx, _) = broadcast::channel::<ServerEvent>(16);
        let mut events_rx = events_tx.subscribe();
        let (audio_tx, _) = broadcast::channel::<RealtimeAudioChunk>(16);
        let (shutdown_tx, join) = spawn_test_loop(transport, cmd_rx, events_tx, audio_tx);
        assert!(matches!(
            events_rx.recv().await,
            Ok(ServerEvent::SessionCreated { session })
                if session.id.as_deref() == Some("s1")
        ));
        assert!(matches!(
            events_rx.recv().await,
            Ok(ServerEvent::SessionUpdated { session })
                if session.input_audio_format == InputAudioFormat::Wav
        ));
        shutdown_tx.send(true).unwrap();
        assert_loop_ok(join).await;
    }

    #[tokio::test]
    async fn run_loop_handles_audio_delta() {
        // Base64-encoded "hello"
        let audio_b64 = base64::engine::general_purpose::STANDARD.encode(b"hello");
        let json = format!(
            r#"{{"type":"response.audio.delta","response_id":"r1","item_id":"i1","delta":"{audio_b64}","event_id":"e1"}}"#
        );
        let transport = ScriptedTransport::new(vec![&json]);
        let (_cmd_tx, cmd_rx) = mpsc::channel::<String>(8);
        let (events_tx, _) = broadcast::channel::<ServerEvent>(16);
        let (audio_tx, _) = broadcast::channel::<RealtimeAudioChunk>(16);
        let mut audio_rx = audio_tx.subscribe();
        let (shutdown_tx, join) = spawn_test_loop(transport, cmd_rx, events_tx, audio_tx);
        let chunk = audio_rx.recv().await.unwrap();
        assert_eq!(chunk.response_id, "r1");
        assert_eq!(chunk.item_id, "i1");
        assert_eq!(chunk.data, Bytes::from_static(b"hello"));
        shutdown_tx.send(true).unwrap();
        assert_loop_ok(join).await;
    }

    #[tokio::test]
    async fn run_loop_handles_error_event() {
        let transport = ScriptedTransport::new(vec![
            r#"{"type":"error","error":{"type":"server_error","code":"server_error","message":"oops"}}"#,
        ]);
        let (_cmd_tx, cmd_rx) = mpsc::channel::<String>(8);
        let (events_tx, _) = broadcast::channel::<ServerEvent>(16);
        let mut events_rx = events_tx.subscribe();
        let (audio_tx, _) = broadcast::channel::<RealtimeAudioChunk>(16);
        let (shutdown_tx, join) = spawn_test_loop(transport, cmd_rx, events_tx, audio_tx);
        assert!(matches!(
            events_rx.recv().await,
            Ok(ServerEvent::Error { .. })
        ));
        shutdown_tx.send(true).unwrap();
        assert_loop_ok(join).await;
    }

    #[tokio::test]
    async fn run_loop_forwards_text_delta_and_done() {
        let transport = ScriptedTransport::new(vec![
            r#"{"type":"response.text.delta","response_id":"r1","item_id":"i1","delta":"hello "}"#,
            r#"{"type":"response.text.done","response_id":"r1","item_id":"i1","text":"hello world"}"#,
        ]);
        let (_cmd_tx, cmd_rx) = mpsc::channel::<String>(8);
        let (events_tx, _) = broadcast::channel::<ServerEvent>(16);
        let mut events_rx = events_tx.subscribe();
        let (audio_tx, _) = broadcast::channel::<RealtimeAudioChunk>(16);
        let (shutdown_tx, join) = spawn_test_loop(transport, cmd_rx, events_tx, audio_tx);

        assert!(matches!(
            events_rx.recv().await,
            Ok(ServerEvent::ResponseTextDelta { delta, .. }) if delta == "hello "
        ));
        assert!(matches!(
            events_rx.recv().await,
            Ok(ServerEvent::ResponseTextDone {
                text: Some(text),
                ..
            }) if text == "hello world"
        ));

        shutdown_tx.send(true).unwrap();
        assert_loop_ok(join).await;
    }

    #[tokio::test]
    async fn run_loop_keeps_both_directions_fair_under_flooding() {
        let received = Arc::new(AtomicUsize::new(0));
        let sent = Arc::new(AtomicUsize::new(0));
        let closed = Arc::new(AtomicBool::new(false));
        let transport = FloodTransport {
            received: Arc::clone(&received),
            sent: Arc::clone(&sent),
            closed: Arc::clone(&closed),
        };
        let (cmd_tx, cmd_rx) = mpsc::channel::<String>(64);
        for _ in 0..32 {
            cmd_tx
                .try_send("{}".into())
                .expect("command queue has capacity");
        }
        drop(cmd_tx);
        let (events_tx, _) = broadcast::channel::<ServerEvent>(16);
        let (audio_tx, _) = broadcast::channel::<RealtimeAudioChunk>(16);

        let (_shutdown_tx, join) = spawn_test_loop(transport, cmd_rx, events_tx, audio_tx);
        assert_loop_ok(join).await;

        assert_eq!(sent.load(Ordering::Relaxed), 32);
        assert!(received.load(Ordering::Relaxed) >= 32);
        assert!(closed.load(Ordering::Relaxed));
    }

    #[tokio::test(start_paused = true)]
    async fn heartbeat_at_idle_boundary_wins_timeout_race() {
        let closed = Arc::new(AtomicBool::new(false));
        let transport = BoundaryHeartbeatTransport {
            delivered: false,
            closed: Arc::clone(&closed),
        };
        let (_cmd_tx, cmd_rx) = mpsc::channel::<String>(8);
        let (events_tx, _) = broadcast::channel::<ServerEvent>(16);
        let mut events_rx = events_tx.subscribe();
        let (audio_tx, _) = broadcast::channel::<RealtimeAudioChunk>(16);
        let (shutdown_tx, join) = spawn_test_loop(transport, cmd_rx, events_tx, audio_tx);

        tokio::task::yield_now().await;
        tokio::time::advance(RealtimeTransportConfig::default().inbound_idle_timeout()).await;
        assert!(matches!(events_rx.recv().await, Ok(ServerEvent::Heartbeat)));
        assert!(
            !join.is_finished(),
            "boundary heartbeat caused a false timeout"
        );

        shutdown_tx.send(true).unwrap();
        assert_loop_ok(join).await;
        assert!(closed.load(Ordering::Relaxed));
    }

    #[tokio::test]
    async fn shutdown_preempts_a_blocked_third_party_send() {
        let send_started = Arc::new(Notify::new());
        let closed = Arc::new(AtomicBool::new(false));
        let transport = BlockingSendTransport {
            send_started: Arc::clone(&send_started),
            closed: Arc::clone(&closed),
        };
        let (cmd_tx, cmd_rx) = mpsc::channel::<String>(8);
        let (events_tx, _) = broadcast::channel::<ServerEvent>(16);
        let (audio_tx, _) = broadcast::channel::<RealtimeAudioChunk>(16);
        let (shutdown_tx, join) = spawn_test_loop(transport, cmd_rx, events_tx, audio_tx);

        cmd_tx.send("blocked frame".into()).await.unwrap();
        send_started.notified().await;
        shutdown_tx.send(true).unwrap();
        assert_loop_ok(join).await;
        assert!(closed.load(Ordering::Relaxed));
    }

    #[tokio::test(start_paused = true)]
    async fn blocked_wire_write_does_not_stall_heartbeat_or_close() {
        let (writer_tx, mut writer_rx) = mpsc::channel::<String>(1);
        let writer_blocked = Arc::new(AtomicBool::new(false));
        let writer_shutdown = Arc::new(Notify::new());
        let closed = Arc::new(AtomicBool::new(false));
        let blocked = Arc::clone(&writer_blocked);
        let stop = Arc::clone(&writer_shutdown);
        let writer = tokio::spawn(async move {
            let _frame = writer_rx.recv().await.expect("writer received frame");
            blocked.store(true, Ordering::Relaxed);
            stop.notified().await;
        });
        let transport = BufferedBlockingWireTransport {
            writer_tx,
            heartbeat_delivered: false,
            writer_shutdown,
            closed: Arc::clone(&closed),
        };
        let (cmd_tx, cmd_rx) = mpsc::channel::<String>(8);
        let (events_tx, _) = broadcast::channel::<ServerEvent>(16);
        let mut events_rx = events_tx.subscribe();
        let (audio_tx, _) = broadcast::channel::<RealtimeAudioChunk>(16);
        let (shutdown_tx, join) = spawn_test_loop(transport, cmd_rx, events_tx, audio_tx);

        cmd_tx.send("queued frame".into()).await.unwrap();
        tokio::task::yield_now().await;
        assert!(
            writer_blocked.load(Ordering::Relaxed),
            "wire writer never entered its blocked state"
        );

        tokio::time::advance(Duration::from_secs(30)).await;
        let heartbeat = events_rx.recv().await;
        assert!(
            matches!(heartbeat, Ok(ServerEvent::Heartbeat)),
            "unexpected event while the wire writer was blocked: {heartbeat:?}"
        );
        assert!(!join.is_finished(), "heartbeat did not keep session alive");

        shutdown_tx.send(true).unwrap();
        assert_loop_ok(join).await;
        writer.await.expect("writer task panicked");
        assert!(closed.load(Ordering::Relaxed));
    }

    #[tokio::test]
    async fn create_then_cancel_preserves_application_fifo_order() {
        let transport = ScriptedTransport::new(vec![]);
        let sent = Arc::clone(&transport.sent);
        let (cmd_tx, cmd_rx) = mpsc::channel::<OutboundCommand>(8);
        let (session_shutdown_tx, _) = watch::channel(false);
        let (session_events_tx, _) = broadcast::channel(8);
        let (session_audio_tx, _) = broadcast::channel(8);
        let (_completion_tx, completion_rx) = watch::channel(None);
        let session = RealtimeSession {
            cmd_tx,
            outbound_budget: Arc::new(Semaphore::new(OUTBOUND_QUEUE_BYTES_MAX)),
            outbound_slots: None,
            outbound_preparation: Arc::new(Semaphore::new(OUTBOUND_PREPARATION_CAPACITY)),
            shutdown_tx: session_shutdown_tx,
            initial_events_rx: Mutex::new(Some(session_events_tx.subscribe())),
            initial_audio_rx: Mutex::new(Some(session_audio_tx.subscribe())),
            events_tx: session_events_tx,
            audio_tx: session_audio_tx,
            completion_rx,
            model_name: "test-realtime".into(),
            input_audio_format: InputAudioFormat::Pcm16,
            transport_config: RealtimeTransportConfig::default(),
            join: tokio::spawn(async { Ok(()) }),
        };
        session
            .send_audio(Bytes::from_static(&[0, 0]))
            .await
            .unwrap();
        session.commit_audio().await.unwrap();
        session.create_response().await.unwrap();
        session.cancel().await.unwrap();

        let (events_tx, _) = broadcast::channel::<ServerEvent>(16);
        let (audio_tx, _) = broadcast::channel::<RealtimeAudioChunk>(16);
        let (shutdown_tx, join) = spawn_test_loop(transport, cmd_rx, events_tx, audio_tx);

        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                if sent.lock().unwrap().len() == 4 {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("outbound commands were not drained");
        shutdown_tx.send(true).unwrap();
        assert_loop_ok(join).await;

        let sent = sent.lock().unwrap();
        let event_types: Vec<_> = sent
            .iter()
            .map(|json| {
                serde_json::from_str::<serde_json::Value>(json).unwrap()["type"]
                    .as_str()
                    .unwrap()
                    .to_owned()
            })
            .collect();
        assert_eq!(
            event_types,
            [
                "input_audio_buffer.append",
                "input_audio_buffer.commit",
                "response.create",
                "response.cancel"
            ]
        );
    }

    #[tokio::test(start_paused = true)]
    async fn injected_max_backlog_preserves_media_barriers_and_reclaims_capacity() {
        const CYCLES: usize = 16;
        const COMMANDS: usize = CYCLES * 4;
        const LANE_MESSAGES: usize = 9;
        const INBOUND_MESSAGES: usize = LANE_MESSAGES * 2 + 1;

        let config = RealtimeTransportConfig::builder()
            .outbound_queue_capacity(COMMANDS)
            .writer_queue_capacity(COMMANDS)
            .event_buffer_capacity(8)
            .audio_buffer_capacity(8)
            .try_build()
            .unwrap();
        let (cmd_tx, cmd_rx) = mpsc::channel(COMMANDS);
        let (session_shutdown_tx, session_shutdown_rx) = watch::channel(false);
        let (session_events_tx, _) = broadcast::channel(config.event_buffer_capacity());
        let (session_audio_tx, _) = broadcast::channel(config.audio_buffer_capacity());
        let (completion_tx, completion_rx) = watch::channel(None);
        let outbound_budget = Arc::new(Semaphore::new(OUTBOUND_QUEUE_BYTES_MAX));
        let outbound_preparation = Arc::new(Semaphore::new(OUTBOUND_PREPARATION_CAPACITY));
        let mut session = RealtimeSession {
            cmd_tx,
            outbound_budget: Arc::clone(&outbound_budget),
            outbound_slots: None,
            outbound_preparation: Arc::clone(&outbound_preparation),
            shutdown_tx: session_shutdown_tx,
            initial_events_rx: Mutex::new(Some(session_events_tx.subscribe())),
            initial_audio_rx: Mutex::new(Some(session_audio_tx.subscribe())),
            events_tx: session_events_tx.clone(),
            audio_tx: session_audio_tx.clone(),
            completion_rx,
            model_name: "test-realtime".into(),
            input_audio_format: InputAudioFormat::Pcm16,
            transport_config: config.clone(),
            join: tokio::spawn(async { Ok(()) }),
        };

        // Fill the largest accepted message-count queue before starting its
        // receiver. Every control command must remain behind its earlier media
        // append; no later cancel is allowed to jump the backlog.
        for cycle in 0..CYCLES {
            let sample = u8::try_from(cycle).unwrap();
            session
                .send_audio(Bytes::from(vec![sample, sample]))
                .await
                .unwrap();
            session.commit_audio().await.unwrap();
            session.create_response().await.unwrap();
            session.cancel().await.unwrap();
        }
        assert_eq!(session.cmd_tx.capacity(), 0);
        assert!(outbound_budget.available_permits() < OUTBOUND_QUEUE_BYTES_MAX);
        assert_eq!(outbound_preparation.available_permits(), 1);

        let (writer_tx, mut writer_rx) = mpsc::channel::<String>(COMMANDS);
        let writer_blocked = Arc::new(Notify::new());
        let release_writer = Arc::new(Notify::new());
        let all_written = Arc::new(Notify::new());
        let written = Arc::new(Mutex::new(Vec::with_capacity(COMMANDS)));
        let writer_stopped = Arc::new(AtomicBool::new(false));
        let (writer_shutdown_tx, mut writer_shutdown_rx) = watch::channel(false);
        let writer_join = {
            let writer_blocked = Arc::clone(&writer_blocked);
            let release_writer = Arc::clone(&release_writer);
            let all_written = Arc::clone(&all_written);
            let written = Arc::clone(&written);
            let writer_stopped = Arc::clone(&writer_stopped);
            tokio::spawn(async move {
                let first = writer_rx.recv().await.expect("writer received no backlog");
                writer_blocked.notify_one();
                tokio::select! {
                    biased;
                    changed = writer_shutdown_rx.changed() => {
                        if changed.is_err() || *writer_shutdown_rx.borrow() {
                            writer_stopped.store(true, Ordering::SeqCst);
                            return;
                        }
                    },
                    () = release_writer.notified() => {},
                }

                written.lock().unwrap().push(first);
                while written.lock().unwrap().len() < COMMANDS {
                    let next = tokio::select! {
                        biased;
                        changed = writer_shutdown_rx.changed() => {
                            if changed.is_err() || *writer_shutdown_rx.borrow() {
                                writer_stopped.store(true, Ordering::SeqCst);
                                return;
                            }
                            continue;
                        },
                        next = writer_rx.recv() => next,
                    };
                    written
                        .lock()
                        .unwrap()
                        .push(next.expect("writer queue closed before the complete backlog"));
                }
                all_written.notify_one();

                loop {
                    if *writer_shutdown_rx.borrow() {
                        break;
                    }
                    if writer_shutdown_rx.changed().await.is_err() {
                        break;
                    }
                }
                writer_stopped.store(true, Ordering::SeqCst);
            })
        };

        let (inbound_tx, inbound_rx) = mpsc::unbounded_channel();
        for index in 0..LANE_MESSAGES {
            inbound_tx
                .send(WsMessage::Text(format!(
                    r#"{{"type":"response.audio.delta","response_id":"r{index}","item_id":"i{index}","delta":"AAA="}}"#
                )))
                .unwrap();
            inbound_tx
                .send(WsMessage::Text(r#"{"type":"heartbeat"}"#.into()))
                .unwrap();
        }
        // This final non-broadcast event is an acknowledgement barrier: once
        // recv returns it, all eighteen preceding event/audio broadcasts have
        // completed before the loop can request another inbound message.
        inbound_tx
            .send(WsMessage::Text(
                r#"{"type":"future.stress.sentinel"}"#.into(),
            ))
            .unwrap();

        let admitted = Arc::new(AtomicUsize::new(0));
        let all_admitted = Arc::new(Notify::new());
        let inbound_processed = Arc::new(AtomicUsize::new(0));
        let all_inbound_processed = Arc::new(Notify::new());
        let close_count = Arc::new(AtomicUsize::new(0));
        let transport = GatedBacklogTransport {
            writer_tx,
            inbound_rx,
            admitted: Arc::clone(&admitted),
            admitted_target: COMMANDS,
            all_admitted: Arc::clone(&all_admitted),
            inbound_processed: Arc::clone(&inbound_processed),
            inbound_target: INBOUND_MESSAGES,
            all_inbound_processed: Arc::clone(&all_inbound_processed),
            writer_shutdown_tx,
            writer_join: Some(writer_join),
            close_count: Arc::clone(&close_count),
        };

        let run_config = config.clone();
        session.join = tokio::spawn(async move {
            let result = run_loop(
                transport,
                cmd_rx,
                session_shutdown_rx,
                session_events_tx,
                session_audio_tx,
                run_config,
            )
            .await;
            completion_tx.send_replace(Some(result.clone()));
            result
        });

        // These are deadlock watchdogs under Tokio's paused clock, not
        // wall-clock performance assertions. Both directions must make exact
        // counted progress while the independent wire writer remains blocked.
        tokio::time::timeout(Duration::from_secs(2), async {
            tokio::join!(
                writer_blocked.notified(),
                all_admitted.notified(),
                all_inbound_processed.notified()
            );
        })
        .await
        .expect("session queues stopped making deterministic progress");
        assert_eq!(admitted.load(Ordering::SeqCst), COMMANDS);
        assert_eq!(inbound_processed.load(Ordering::SeqCst), INBOUND_MESSAGES);
        assert_eq!(session.cmd_tx.capacity(), COMMANDS);
        assert_eq!(
            outbound_budget.available_permits(),
            OUTBOUND_QUEUE_BYTES_MAX
        );
        assert_eq!(outbound_preparation.available_permits(), 1);
        assert!(written.lock().unwrap().is_empty());

        // Both bounded broadcast lanes fail observably after nine unconsumed
        // values in an eight-slot buffer; neither silently drops data.
        let mut events = session.events();
        let event_lag = events
            .next()
            .await
            .expect("event stream ended before reporting lag")
            .expect_err("event overflow was silently ignored");
        assert!(event_lag.message().contains("lost 1 message"));
        assert!(events.next().await.is_none());
        drop(events);

        let mut audio = session.audio_stream();
        let audio_lag = audio
            .next()
            .await
            .expect("audio stream ended before reporting lag")
            .expect_err("audio overflow was silently ignored");
        assert!(audio_lag.message().contains("lost 1 message"));
        assert!(audio.next().await.is_none());
        drop(audio);

        release_writer.notify_one();
        tokio::time::timeout(Duration::from_secs(2), all_written.notified())
            .await
            .expect("released writer did not drain the complete backlog");

        {
            let frames = written.lock().unwrap();
            assert_eq!(frames.len(), COMMANDS);
            for (cycle, group) in frames.as_slice().as_chunks::<4>().0.iter().enumerate() {
                let values = group
                    .iter()
                    .map(|frame| serde_json::from_str::<serde_json::Value>(frame).unwrap())
                    .collect::<Vec<_>>();
                assert_eq!(values[0]["type"], "input_audio_buffer.append");
                assert_eq!(values[1]["type"], "input_audio_buffer.commit");
                assert_eq!(values[2]["type"], "response.create");
                assert_eq!(values[3]["type"], "response.cancel");
                let audio = values[0]["audio"].as_str().unwrap();
                assert_eq!(
                    base64::engine::general_purpose::STANDARD
                        .decode(audio)
                        .unwrap(),
                    vec![u8::try_from(cycle).unwrap(); 2]
                );
            }
        }

        tokio::time::timeout(Duration::from_secs(2), session.close())
            .await
            .expect("session close exceeded its deterministic watchdog")
            .unwrap();
        assert_eq!(close_count.load(Ordering::SeqCst), 1);
        assert!(writer_stopped.load(Ordering::SeqCst));

        // Keep the inbound producer alive until close so an exhausted test
        // script cannot masquerade as a peer disconnect.
        drop(inbound_tx);
    }

    #[tokio::test]
    async fn outbound_budget_counts_bytes_and_releases_on_drop() {
        let budget = Arc::new(Semaphore::new(5));
        let first = budget_outbound_command(Arc::clone(&budget), None, "1234".into())
            .await
            .unwrap();
        let second = budget_outbound_command(Arc::clone(&budget), None, "12".into());
        tokio::pin!(second);
        assert!(
            tokio::time::timeout(Duration::from_millis(20), &mut second)
                .await
                .is_err(),
            "second payload ignored the aggregate byte budget"
        );

        drop(first);
        let second = tokio::time::timeout(Duration::from_secs(1), second)
            .await
            .expect("released byte permits were not reusable")
            .unwrap();
        drop(second);
        assert_eq!(budget.available_permits(), 5);
    }

    struct RetainingBuiltInIo {
        writer_tx: mpsc::UnboundedSender<(String, SessionFrameBudget)>,
        closed: Arc<AtomicBool>,
    }

    #[async_trait]
    impl BuiltInSessionIo for RetainingBuiltInIo {
        fn enqueue_session_text(
            &mut self,
            json: String,
            budget: SessionFrameBudget,
        ) -> ZaiResult<()> {
            self.writer_tx
                .send((json, budget))
                .map_err(|_| protocol_error("test built-in writer is closed"))
        }

        async fn recv(&mut self) -> ZaiResult<Option<WsMessage>> {
            std::future::pending().await
        }

        async fn close(&mut self) -> ZaiResult<()> {
            self.closed.store(true, Ordering::SeqCst);
            Ok(())
        }
    }

    #[tokio::test(start_paused = true)]
    async fn builtin_adapter_transfers_admission_until_writer_releases_frame() {
        let timeout = Duration::from_secs(5);
        let config = RealtimeTransportConfig::builder()
            .outbound_queue_timeout(timeout)
            .outbound_queue_capacity(8)
            .writer_queue_capacity(1)
            .try_build()
            .unwrap();
        let (writer_tx, mut writer_rx) = mpsc::unbounded_channel();
        let closed = Arc::new(AtomicBool::new(false));
        let io = RetainingBuiltInIo {
            writer_tx,
            closed: Arc::clone(&closed),
        };
        let session = spawn_session(
            BuiltInSessionTransport(io),
            "test-realtime".into(),
            InputAudioFormat::Pcm16,
            config.clone(),
            Some(built_in_pipeline_capacity(&config)),
        );

        session.create_response().await.unwrap();
        let first = writer_rx
            .recv()
            .await
            .expect("built-in adapter did not transfer the first command");

        let error = {
            let second = session.cancel();
            tokio::pin!(second);
            tokio::select! {
                biased;
                result = &mut second => panic!("writer-held slot did not block admission: {result:?}"),
                () = tokio::task::yield_now() => {},
            }
            tokio::time::advance(timeout).await;
            second
                .await
                .expect_err("second command bypassed the writer-held slot")
        };
        assert!(error.message().contains("outbound admission timed out"));
        assert!(
            !session.join.is_finished(),
            "internal writer pressure terminated the session"
        );

        drop(first);
        session.create_response().await.unwrap();
        let third = writer_rx
            .recv()
            .await
            .expect("released end-to-end slot was not reusable");
        drop(third);
        session.close().await.unwrap();
        assert!(closed.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn outbound_slot_counts_the_complete_builtin_pipeline() {
        let bytes = Arc::new(Semaphore::new(OUTBOUND_QUEUE_BYTES_MAX));
        let slots = Arc::new(Semaphore::new(1));
        let first =
            budget_outbound_command(Arc::clone(&bytes), Some(Arc::clone(&slots)), "first".into())
                .await
                .unwrap();
        let second = budget_outbound_command(
            Arc::clone(&bytes),
            Some(Arc::clone(&slots)),
            "second".into(),
        );
        tokio::pin!(second);
        assert!(
            tokio::time::timeout(Duration::from_millis(20), &mut second)
                .await
                .is_err(),
            "a second command escaped the end-to-end message-count bound"
        );

        drop(first);
        let second = tokio::time::timeout(Duration::from_secs(1), second)
            .await
            .expect("released end-to-end slot was not reusable")
            .unwrap();
        drop(second);
        assert_eq!(slots.available_permits(), 1);
        assert_eq!(bytes.available_permits(), OUTBOUND_QUEUE_BYTES_MAX);
    }

    #[test]
    fn builtin_pipeline_uses_the_stricter_message_count_bound() {
        let config = RealtimeTransportConfig::builder()
            .outbound_queue_capacity(64)
            .writer_queue_capacity(1)
            .try_build()
            .unwrap();
        assert_eq!(built_in_pipeline_capacity(&config), 1);

        let config = RealtimeTransportConfig::builder()
            .outbound_queue_capacity(1)
            .writer_queue_capacity(64)
            .try_build()
            .unwrap();
        assert_eq!(built_in_pipeline_capacity(&config), 1);
    }

    #[test]
    fn outbound_budget_rejects_a_message_larger_than_the_fixed_byte_cap() {
        let oversized = "x".repeat(OUTBOUND_QUEUE_BYTES_MAX + 1);
        let error = outbound_message_permits(&oversized)
            .expect_err("oversized message could wait forever on an impossible permit count");
        assert!(error.message().contains("queue budget"));
        assert_eq!(
            outbound_message_permits(&"x".repeat(OUTBOUND_QUEUE_BYTES_MAX)).unwrap(),
            OUTBOUND_QUEUE_BYTES_MAX as u32
        );
    }

    fn unpolled_test_session(
        config: RealtimeTransportConfig,
    ) -> (RealtimeSession, mpsc::Receiver<OutboundCommand>) {
        let (cmd_tx, cmd_rx) = mpsc::channel(config.outbound_queue_capacity());
        let (shutdown_tx, _) = watch::channel(false);
        let (events_tx, _) = broadcast::channel(config.event_buffer_capacity());
        let (audio_tx, _) = broadcast::channel(config.audio_buffer_capacity());
        let (_completion_tx, completion_rx) = watch::channel(None);
        (
            RealtimeSession {
                cmd_tx,
                outbound_budget: Arc::new(Semaphore::new(OUTBOUND_QUEUE_BYTES_MAX)),
                outbound_slots: None,
                outbound_preparation: Arc::new(Semaphore::new(OUTBOUND_PREPARATION_CAPACITY)),
                shutdown_tx,
                initial_events_rx: Mutex::new(Some(events_tx.subscribe())),
                initial_audio_rx: Mutex::new(Some(audio_tx.subscribe())),
                events_tx,
                audio_tx,
                completion_rx,
                model_name: "test-realtime".into(),
                input_audio_format: InputAudioFormat::Pcm16,
                transport_config: config,
                join: tokio::spawn(async { Ok(()) }),
            },
            cmd_rx,
        )
    }

    #[tokio::test]
    async fn zero_outbound_deadline_fails_fast_when_the_command_queue_is_full() {
        let config = RealtimeTransportConfig::builder()
            .outbound_queue_timeout(Duration::ZERO)
            .outbound_queue_capacity(1)
            .try_build()
            .unwrap();
        let (session, _cmd_rx) = unpolled_test_session(config);

        session.create_response().await.unwrap();
        let error = session
            .cancel()
            .await
            .expect_err("zero admission deadline waited on a full queue");
        assert!(error.message().contains("outbound admission timed out"));
    }

    #[tokio::test]
    async fn outbound_deadline_accounts_for_synchronous_preparation_time() {
        let config = RealtimeTransportConfig::builder()
            .outbound_queue_timeout(Duration::from_millis(5))
            .try_build()
            .unwrap();
        let (session, _cmd_rx) = unpolled_test_session(config);

        let error = session
            .prepare_and_dispatch(|| {
                std::thread::sleep(Duration::from_millis(10));
                Ok(ClientEvent::ResponseCreate {
                    client_timestamp: Some(1),
                })
            })
            .await
            .expect_err("synchronous preparation escaped the absolute admission deadline");
        assert!(error.message().contains("outbound admission timed out"));
    }

    #[tokio::test(start_paused = true)]
    async fn command_released_at_the_deadline_is_not_published_late() {
        let queue_timeout = Duration::from_secs(5);
        let config = RealtimeTransportConfig::builder()
            .outbound_queue_timeout(queue_timeout)
            .outbound_queue_capacity(1)
            .try_build()
            .unwrap();
        let (session, mut cmd_rx) = unpolled_test_session(config);
        session.create_response().await.unwrap();

        let second = session.cancel();
        tokio::pin!(second);
        tokio::select! {
            biased;
            result = &mut second => panic!("second command did not wait: {result:?}"),
            () = tokio::task::yield_now() => {},
        }

        tokio::time::advance(queue_timeout).await;
        drop(cmd_rx.recv().await.expect("first command disappeared"));
        let error = second
            .await
            .expect_err("a command was published after its absolute admission deadline");
        assert!(error.message().contains("outbound admission timed out"));
        assert!(matches!(
            cmd_rx.try_recv(),
            Err(mpsc::error::TryRecvError::Empty)
        ));
    }

    #[tokio::test]
    async fn outbound_preparation_is_bounded_before_serialization() {
        let (cmd_tx, mut cmd_rx) = mpsc::channel::<OutboundCommand>(8);
        let (session_shutdown_tx, _) = watch::channel(false);
        let (session_events_tx, _) = broadcast::channel(8);
        let (session_audio_tx, _) = broadcast::channel(8);
        let (_completion_tx, completion_rx) = watch::channel(None);
        let budget = Arc::new(Semaphore::new(OUTBOUND_QUEUE_BYTES_MAX));
        let session = RealtimeSession {
            cmd_tx,
            outbound_budget: Arc::clone(&budget),
            outbound_slots: None,
            outbound_preparation: Arc::new(Semaphore::new(OUTBOUND_PREPARATION_CAPACITY)),
            shutdown_tx: session_shutdown_tx,
            initial_events_rx: Mutex::new(Some(session_events_tx.subscribe())),
            initial_audio_rx: Mutex::new(Some(session_audio_tx.subscribe())),
            events_tx: session_events_tx,
            audio_tx: session_audio_tx,
            completion_rx,
            model_name: "test-realtime".into(),
            input_audio_format: InputAudioFormat::Pcm16,
            transport_config: RealtimeTransportConfig::default(),
            join: tokio::spawn(async { Ok(()) }),
        };
        let blocked_budget = Arc::clone(&budget)
            .acquire_many_owned(OUTBOUND_QUEUE_BYTES_MAX as u32)
            .await
            .unwrap();
        let prepared = Arc::new(std::sync::atomic::AtomicUsize::new(0));

        let first = session.prepare_and_dispatch({
            let prepared = Arc::clone(&prepared);
            move || {
                prepared.fetch_add(1, Ordering::SeqCst);
                Ok(ClientEvent::ResponseCreate {
                    client_timestamp: Some(1),
                })
            }
        });
        tokio::pin!(first);
        assert!(
            tokio::time::timeout(Duration::from_millis(20), &mut first)
                .await
                .is_err()
        );
        assert_eq!(prepared.load(Ordering::SeqCst), 1);

        let second = session.prepare_and_dispatch({
            let prepared = Arc::clone(&prepared);
            move || {
                prepared.fetch_add(1, Ordering::SeqCst);
                Ok(ClientEvent::ResponseCancel {
                    client_timestamp: Some(2),
                })
            }
        });
        tokio::pin!(second);
        assert!(
            tokio::time::timeout(Duration::from_millis(20), &mut second)
                .await
                .is_err()
        );
        assert_eq!(
            prepared.load(Ordering::SeqCst),
            1,
            "the second event was prepared outside the admission bound"
        );

        drop(blocked_budget);
        let (first_result, second_result) = tokio::join!(first, second);
        first_result.unwrap();
        second_result.unwrap();
        assert_eq!(prepared.load(Ordering::SeqCst), 2);
        assert!(budget.available_permits() < OUTBOUND_QUEUE_BYTES_MAX);

        let first_command = cmd_rx.recv().await.unwrap();
        let second_command = cmd_rx.recv().await.unwrap();
        drop((first_command, second_command));
        assert_eq!(budget.available_permits(), OUTBOUND_QUEUE_BYTES_MAX);
    }

    #[tokio::test]
    async fn run_loop_sends_client_event() {
        let transport = ScriptedTransport::new(vec![]);
        let sent = Arc::clone(&transport.sent);
        let (cmd_tx, cmd_rx) = mpsc::channel::<String>(8);
        let (events_tx, _) = broadcast::channel::<ServerEvent>(16);
        let (audio_tx, _) = broadcast::channel::<RealtimeAudioChunk>(16);
        let (_shutdown_tx, join) = spawn_test_loop(transport, cmd_rx, events_tx, audio_tx);
        cmd_tx
            .send(
                serialize_event(&ClientEvent::ResponseCreate {
                    client_timestamp: None,
                })
                .unwrap(),
            )
            .await
            .unwrap();
        drop(cmd_tx);
        assert_loop_ok(join).await;
        assert_eq!(sent.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn run_loop_shutdown_signal_terminates() {
        let transport = ScriptedTransport::new(vec![]);
        let (_cmd_tx, cmd_rx) = mpsc::channel::<String>(8);
        let (events_tx, _) = broadcast::channel::<ServerEvent>(16);
        let (audio_tx, _) = broadcast::channel::<RealtimeAudioChunk>(16);
        let (shutdown_tx, join) = spawn_test_loop(transport, cmd_rx, events_tx, audio_tx);
        shutdown_tx.send(true).unwrap();
        assert_loop_ok(join).await;
    }

    #[tokio::test]
    async fn run_loop_peer_disconnect_terminates() {
        // Empty message queue → recv returns None → peer disconnected
        let transport = ScriptedTransport::disconnecting();
        let (_cmd_tx, cmd_rx) = mpsc::channel::<String>(8);
        let (events_tx, _) = broadcast::channel::<ServerEvent>(16);
        let (audio_tx, _) = broadcast::channel::<RealtimeAudioChunk>(16);
        let (_shutdown_tx, join) = spawn_test_loop(transport, cmd_rx, events_tx, audio_tx);
        assert_loop_ok(join).await;
    }

    #[tokio::test]
    async fn unknown_event_is_ignored_without_hiding_following_known_event() {
        let transport = ScriptedTransport::new(vec![
            r#"{"type":"future.event","payload":true}"#,
            r#"{"type":"heartbeat"}"#,
        ]);
        let (_cmd_tx, cmd_rx) = mpsc::channel::<String>(8);
        let (events_tx, _) = broadcast::channel::<ServerEvent>(16);
        let mut events_rx = events_tx.subscribe();
        let (audio_tx, _) = broadcast::channel::<RealtimeAudioChunk>(16);
        let (shutdown_tx, join) = spawn_test_loop(transport, cmd_rx, events_tx, audio_tx);

        assert!(matches!(events_rx.recv().await, Ok(ServerEvent::Heartbeat)));
        shutdown_tx.send(true).unwrap();
        assert_loop_ok(join).await;
    }

    #[tokio::test]
    async fn unsupported_known_event_is_observable_and_does_not_close_the_session() {
        let transport = ScriptedTransport::new(vec![
            r#"{"type":"session.updated","session":{"input_audio_format":"wav","output_audio_format":"pcm","voice":"future_voice","turn_detection":{"type":"client_vad"}}}"#,
            r#"{"type":"heartbeat"}"#,
        ]);
        let closed = Arc::clone(&transport.closed);
        let (_cmd_tx, cmd_rx) = mpsc::channel::<String>(8);
        let (events_tx, _) = broadcast::channel::<ServerEvent>(16);
        let mut events_rx = events_tx.subscribe();
        let (audio_tx, _) = broadcast::channel::<RealtimeAudioChunk>(16);
        let (shutdown_tx, join) = spawn_test_loop(transport, cmd_rx, events_tx, audio_tx);

        match events_rx.recv().await {
            Ok(ServerEvent::UnsupportedKnown { event_type, raw }) => {
                assert_eq!(event_type, "session.updated");
                assert_eq!(raw["session"]["voice"], "future_voice");
            },
            event => panic!("unsupported known event was not observable: {event:?}"),
        }
        assert!(matches!(events_rx.recv().await, Ok(ServerEvent::Heartbeat)));
        assert!(!*closed.lock().unwrap());

        shutdown_tx.send(true).unwrap();
        assert_loop_ok(join).await;
    }

    #[tokio::test]
    async fn unsupported_nested_value_does_not_hide_a_malformed_sibling() {
        let transport = ScriptedTransport::new(vec![
            r#"{"type":"session.updated","session":{"input_audio_format":"wav","output_audio_format":"pcm","voice":"future_voice","turn_detection":{"type":"client_vad"},"temperature":"hot"}}"#,
        ]);
        let closed = Arc::clone(&transport.closed);
        let (_cmd_tx, cmd_rx) = mpsc::channel::<String>(8);
        let (events_tx, _) = broadcast::channel::<ServerEvent>(16);
        let (audio_tx, _) = broadcast::channel::<RealtimeAudioChunk>(16);
        let (_shutdown_tx, join) = spawn_test_loop(transport, cmd_rx, events_tx, audio_tx);
        let error = join
            .await
            .expect("run_loop task panicked")
            .expect_err("a malformed sibling was hidden by compatibility decoding");

        assert!(error.message().contains("malformed realtime server event"));
        assert!(*closed.lock().unwrap());
    }

    #[tokio::test]
    async fn malformed_known_event_closes_session() {
        let transport = ScriptedTransport::new(vec![
            r#"{"type":"response.text.delta","response_id":"r1","delta":"missing item"}"#,
        ]);
        let closed = Arc::clone(&transport.closed);
        let (_cmd_tx, cmd_rx) = mpsc::channel::<String>(8);
        let (events_tx, _) = broadcast::channel::<ServerEvent>(16);
        let (audio_tx, _) = broadcast::channel::<RealtimeAudioChunk>(16);
        let (_shutdown_tx, join) = spawn_test_loop(transport, cmd_rx, events_tx, audio_tx);
        let error = join
            .await
            .expect("run_loop task panicked")
            .expect_err("malformed known event was silently ignored");

        assert!(error.message().contains("malformed realtime server event"));
        assert!(*closed.lock().unwrap());
    }

    #[tokio::test(start_paused = true)]
    async fn run_loop_closes_half_open_session_after_missed_heartbeats() {
        let transport = ScriptedTransport::new(vec![]);
        let closed = Arc::clone(&transport.closed);
        // Keep the sender alive so only the inbound idle deadline can stop the
        // loop; this models a half-open connection with a healthy local task.
        let (_cmd_tx, cmd_rx) = mpsc::channel::<String>(8);
        let (events_tx, _) = broadcast::channel::<ServerEvent>(16);
        let (audio_tx, _) = broadcast::channel::<RealtimeAudioChunk>(16);
        let (_shutdown_tx, join) = spawn_test_loop(transport, cmd_rx, events_tx, audio_tx);

        tokio::task::yield_now().await;
        tokio::time::advance(RealtimeTransportConfig::default().inbound_idle_timeout()).await;
        let error = join
            .await
            .expect("run_loop task panicked")
            .expect_err("half-open session did not time out");

        assert!(error.message().contains("inbound heartbeat timed out"));
        assert!(*closed.lock().unwrap());
    }

    #[test]
    fn new_event_id_format() {
        let first = new_event_id();
        let second = new_event_id();
        assert!(first.starts_with("evt_"));
        assert_eq!(first.len(), 36);
        assert_ne!(first, second);
    }

    #[test]
    fn session_queues_match_default_policy_capacity() {
        let config = RealtimeTransportConfig::default();
        assert_eq!(config.outbound_queue_capacity(), 8);
        assert_eq!(config.event_buffer_capacity(), 8);
        assert_eq!(config.audio_buffer_capacity(), 8);
    }

    #[test]
    fn oversized_event_is_rejected_before_enqueue() {
        let event = ClientEvent::ConversationItemCreate {
            event_id: None,
            item: super::super::protocol::RealtimeConversationItem::user_text(
                "x".repeat(WS_MESSAGE_MAX as usize),
            ),
        };
        assert!(serialize_event(&event).is_err());
    }

    #[test]
    fn session_config_validates_numeric_limits() {
        let mut config = SessionConfig {
            temperature: Some(f64::NAN),
            ..SessionConfig::default()
        };
        assert!(validate_session_config(&config).is_err());

        config.temperature = Some(0.5);
        config.max_response_output_tokens = Some(0);
        assert!(validate_session_config(&config).is_err());

        config.max_response_output_tokens = Some(1025);
        assert!(validate_session_config(&config).is_err());

        config.max_response_output_tokens = Some(1024);
        assert!(validate_session_config(&config).is_ok());
    }

    #[test]
    fn session_config_validates_modalities_vad_and_tools() {
        let mut config = SessionConfig {
            modalities: Vec::new(),
            ..SessionConfig::default()
        };
        assert!(validate_session_config(&config).is_err());

        config.modalities = vec![RealtimeModality::Text, RealtimeModality::Text];
        assert!(validate_session_config(&config).is_err());

        config.modalities = vec![RealtimeModality::Text, RealtimeModality::Audio];
        config.turn_detection.create_response = Some(true);
        assert!(validate_session_config(&config).is_err());

        config.turn_detection.type_ = TurnDetectionType::ServerVad;
        config.turn_detection.threshold = Some(f64::NAN);
        assert!(validate_session_config(&config).is_err());

        config.turn_detection.threshold = Some(0.5);
        config.tools = vec![RealtimeTool::function(
            "weather",
            "Get weather",
            serde_json::json!({"type": "object"}),
        )];
        config.beta_fields.chat_mode = Some(ChatMode::VideoPassive);
        assert!(validate_session_config(&config).is_err());

        config.beta_fields.chat_mode = Some(ChatMode::Audio);
        config.tools.push(config.tools[0].clone());
        assert!(validate_session_config(&config).is_err());

        config.tools.pop();
        assert!(validate_session_config(&config).is_ok());
    }

    #[test]
    fn session_builder_preflight_validates_config_and_auth_without_network() {
        let builder = SessionBuilder::new(
            Arc::new(ApiSecret::new("abcdefghij.0123456789abcdef")),
            AuthMode::Bearer,
            "wss://example.com/realtime".to_owned(),
            "glm-realtime-flash".to_owned(),
            RealtimeTransportConfig::default(),
        );
        builder.validate().unwrap();

        let invalid_config = SessionBuilder::new(
            Arc::new(ApiSecret::new("abcdefghij.0123456789abcdef")),
            AuthMode::Bearer,
            "wss://example.com/realtime".to_owned(),
            "glm-realtime-flash".to_owned(),
            RealtimeTransportConfig::default(),
        )
        .temperature(f64::NAN);
        assert!(invalid_config.validate().is_err());

        let invalid_key = SessionBuilder::new(
            Arc::new(ApiSecret::new("not-a-provider-key")),
            AuthMode::Bearer,
            "wss://example.com/realtime".to_owned(),
            "glm-realtime-flash".to_owned(),
            RealtimeTransportConfig::default(),
        );
        assert!(invalid_key.validate().is_err());

        let invalid_jwt_ttl = SessionBuilder::new(
            Arc::new(ApiSecret::new("abcdefghij.0123456789abcdef")),
            AuthMode::Jwt { ttl_seconds: 0 },
            "wss://example.com/realtime".to_owned(),
            "glm-realtime-flash".to_owned(),
            RealtimeTransportConfig::default(),
        );
        assert!(invalid_jwt_ttl.validate().is_err());
    }

    #[tokio::test]
    async fn observable_stream_reports_lag_and_background_failure() {
        let (events_tx, events_rx) = broadcast::channel(2);
        let (_completion_tx, completion_rx) = watch::channel(None);
        let mut events = observable_broadcast_stream(events_rx, completion_rx, "test events");
        for value in 0..3 {
            events_tx.send(value).unwrap();
        }
        let lag = events
            .next()
            .await
            .expect("stream ended")
            .expect_err("lag was silently discarded");
        assert!(lag.message().contains("lost 1 message"));
        assert!(
            events.next().await.is_none(),
            "a corrupted stream continued after reporting lag"
        );

        let (_events_tx, events_rx) = broadcast::channel::<u8>(2);
        let (completion_tx, completion_rx) = watch::channel(None);
        let mut events = observable_broadcast_stream(events_rx, completion_rx, "test events");
        completion_tx.send_replace(Some(Err(protocol_error("background failed"))));
        let failure = events
            .next()
            .await
            .expect("stream ended before reporting failure")
            .expect_err("background failure was hidden");
        assert!(failure.message().contains("background failed"));
        assert!(events.next().await.is_none());

        let (events_tx, events_rx) = broadcast::channel::<u8>(2);
        let (completion_tx, completion_rx) = watch::channel(None);
        let mut events = observable_broadcast_stream(events_rx, completion_rx, "test events");
        events_tx.send(9).unwrap();
        drop(completion_tx);
        assert_eq!(events.next().await.unwrap().unwrap(), 9);
        let failure = events
            .next()
            .await
            .expect("stream ended before reporting task loss")
            .expect_err("missing completion status was hidden");
        assert!(failure.message().contains("without a completion status"));
    }

    #[test]
    fn first_subscription_keeps_pre_subscription_backlog() {
        let (events_tx, initial_rx) =
            broadcast::channel(RealtimeTransportConfig::default().event_buffer_capacity());
        let initial = Mutex::new(Some(initial_rx));
        events_tx.send(7_u8).unwrap();

        let mut first = subscribe_with_initial_backlog(&events_tx, &initial);
        assert_eq!(first.try_recv().unwrap(), 7);

        let mut second = subscribe_with_initial_backlog(&events_tx, &initial);
        assert!(matches!(
            second.try_recv(),
            Err(broadcast::error::TryRecvError::Empty)
        ));
    }
}

#[cfg(test)]
mod injected_builder_tests {
    use super::*;
    use async_trait::async_trait;
    use futures_util::StreamExt as _;
    use std::{
        future::pending,
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        },
        time::Duration,
    };

    fn builder() -> SessionBuilder {
        SessionBuilder::new(
            Arc::new(ApiSecret::new("unused-by-injected-transport")),
            AuthMode::Bearer,
            "wss://unused.invalid/realtime".to_owned(),
            "glm-realtime-flash".to_owned(),
            RealtimeTransportConfig::default(),
        )
    }

    struct PendingLifecycleTransport {
        sends: Arc<AtomicUsize>,
        closes: Arc<AtomicUsize>,
    }

    #[async_trait]
    impl RealtimeTransport for PendingLifecycleTransport {
        async fn send(&mut self, _msg: String) -> ZaiResult<()> {
            self.sends.fetch_add(1, Ordering::SeqCst);
            pending().await
        }

        async fn recv(&mut self) -> ZaiResult<Option<WsMessage>> {
            pending().await
        }

        async fn close(&mut self) -> ZaiResult<()> {
            self.closes.fetch_add(1, Ordering::SeqCst);
            pending().await
        }
    }

    #[tokio::test(start_paused = true)]
    async fn pending_initial_update_and_close_have_hard_deadlines() {
        let sends = Arc::new(AtomicUsize::new(0));
        let closes = Arc::new(AtomicUsize::new(0));
        let transport = PendingLifecycleTransport {
            sends: Arc::clone(&sends),
            closes: Arc::clone(&closes),
        };
        let build = tokio::spawn(builder().build_with_transport(transport));

        tokio::task::yield_now().await;
        assert_eq!(sends.load(Ordering::SeqCst), 1);
        tokio::time::advance(RealtimeTransportConfig::default().initial_update_timeout()).await;
        while closes.load(Ordering::SeqCst) == 0 {
            tokio::task::yield_now().await;
        }
        tokio::time::advance(RealtimeTransportConfig::default().transport_close_timeout()).await;

        let error = build
            .await
            .expect("injected build task panicked")
            .err()
            .expect("pending initial update unexpectedly succeeded");
        assert!(
            error
                .message()
                .contains("initial session.update send timed out"),
            "close timeout replaced the primary error: {error}"
        );
        assert_eq!(closes.load(Ordering::SeqCst), 1);
    }

    struct PendingSendTransport {
        sends: Arc<AtomicUsize>,
        closes: Arc<AtomicUsize>,
    }

    #[async_trait]
    impl RealtimeTransport for PendingSendTransport {
        async fn send(&mut self, _msg: String) -> ZaiResult<()> {
            self.sends.fetch_add(1, Ordering::SeqCst);
            pending().await
        }

        async fn recv(&mut self) -> ZaiResult<Option<WsMessage>> {
            pending().await
        }

        async fn close(&mut self) -> ZaiResult<()> {
            self.closes.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    #[tokio::test(start_paused = true)]
    async fn pending_regular_send_cannot_suspend_inbound_forever() {
        let sends = Arc::new(AtomicUsize::new(0));
        let closes = Arc::new(AtomicUsize::new(0));
        let transport = PendingSendTransport {
            sends: Arc::clone(&sends),
            closes: Arc::clone(&closes),
        };
        let mut transport = InjectedSessionTransport(transport);
        let (_shutdown_tx, mut shutdown_rx) = watch::channel(false);
        let send = tokio::spawn(async move {
            handle_outbound(
                &mut transport,
                Some(OutboundCommand::from("{}".to_owned())),
                &mut shutdown_rx,
                &RealtimeTransportConfig::default(),
            )
            .await
        });

        tokio::task::yield_now().await;
        assert_eq!(sends.load(Ordering::SeqCst), 1);
        tokio::time::advance(RealtimeTransportConfig::default().transport_send_timeout()).await;
        let error = send
            .await
            .expect("outbound task panicked")
            .expect_err("pending third-party send unexpectedly succeeded");
        assert!(error.message().contains("transport send timed out"));
        assert_eq!(closes.load(Ordering::SeqCst), 1);
    }

    struct CountingTransport {
        sends: Arc<AtomicUsize>,
        closes: Arc<AtomicUsize>,
    }

    #[async_trait]
    impl RealtimeTransport for CountingTransport {
        async fn send(&mut self, _msg: String) -> ZaiResult<()> {
            self.sends.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        async fn recv(&mut self) -> ZaiResult<Option<WsMessage>> {
            pending().await
        }

        async fn close(&mut self) -> ZaiResult<()> {
            self.closes.fetch_add(1, Ordering::SeqCst);
            Err(protocol_error("secondary injected close failure"))
        }
    }

    struct PeerDisconnectCloseFailureTransport {
        closes: Arc<AtomicUsize>,
    }

    #[async_trait]
    impl RealtimeTransport for PeerDisconnectCloseFailureTransport {
        async fn send(&mut self, _msg: String) -> ZaiResult<()> {
            Ok(())
        }

        async fn recv(&mut self) -> ZaiResult<Option<WsMessage>> {
            Ok(None)
        }

        async fn close(&mut self) -> ZaiResult<()> {
            self.closes.fetch_add(1, Ordering::SeqCst);
            Err(protocol_error("peer-disconnect close failure"))
        }
    }

    #[tokio::test]
    async fn peer_disconnect_preserves_transport_close_failure() {
        let closes = Arc::new(AtomicUsize::new(0));
        let session = builder()
            .build_with_transport(PeerDisconnectCloseFailureTransport {
                closes: Arc::clone(&closes),
            })
            .await
            .expect("injected session build failed");

        let mut events = session.events();
        let observed = events
            .next()
            .await
            .expect("event stream ended before reporting close failure")
            .expect_err("event stream hid peer-disconnect close failure");
        assert!(observed.message().contains("peer-disconnect close failure"));
        assert!(events.next().await.is_none());
        drop(events);

        let error = session
            .close()
            .await
            .expect_err("peer-disconnect close failure was hidden");
        assert!(error.message().contains("peer-disconnect close failure"));
        assert_eq!(closes.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn invalid_session_is_closed_without_sending_and_keeps_validation_error() {
        let sends = Arc::new(AtomicUsize::new(0));
        let closes = Arc::new(AtomicUsize::new(0));
        let transport = CountingTransport {
            sends: Arc::clone(&sends),
            closes: Arc::clone(&closes),
        };

        let error = builder()
            .temperature(f64::NAN)
            .build_with_transport(transport)
            .await
            .err()
            .expect("invalid injected session unexpectedly succeeded");
        assert!(error.message().contains("temperature"));
        assert_eq!(sends.load(Ordering::SeqCst), 0);
        assert_eq!(closes.load(Ordering::SeqCst), 1);
    }

    struct EarlyEventTransport {
        delivered: bool,
        closes: Arc<AtomicUsize>,
    }

    #[async_trait]
    impl RealtimeTransport for EarlyEventTransport {
        async fn send(&mut self, _msg: String) -> ZaiResult<()> {
            Ok(())
        }

        async fn recv(&mut self) -> ZaiResult<Option<WsMessage>> {
            if self.delivered {
                return pending().await;
            }
            self.delivered = true;
            Ok(Some(WsMessage::Text(r#"{"type":"heartbeat"}"#.into())))
        }

        async fn close(&mut self) -> ZaiResult<()> {
            self.closes.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    #[tokio::test]
    async fn injected_early_event_is_kept_for_the_first_subscriber() {
        let closes = Arc::new(AtomicUsize::new(0));
        let session = builder()
            .build_with_transport(EarlyEventTransport {
                delivered: false,
                closes: Arc::clone(&closes),
            })
            .await
            .unwrap();

        let mut events = session.events();
        let event = tokio::time::timeout(Duration::from_secs(2), events.next())
            .await
            .expect("early injected event was not retained")
            .expect("event stream ended before the early event")
            .unwrap();
        assert!(matches!(event, ServerEvent::Heartbeat));
        drop(events);
        session.close().await.unwrap();
        assert_eq!(closes.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn oversized_injected_text_is_rejected_before_deserialization() {
        let text = "x".repeat(WS_MESSAGE_MAX as usize + 1);
        let error = match decode_server_frame(&text) {
            Ok(_) => panic!("oversized injected text unexpectedly decoded"),
            Err(error) => error,
        };
        assert!(error.message().contains("inbound message exceeds"));
    }

    #[test]
    fn oversized_audio_delta_is_rejected_before_base64_allocation() {
        let encoded_max = base64::encoded_len(REALTIME_AUDIO_FRAME_MAX as usize, true).unwrap();
        let encoded = "A".repeat(encoded_max + 4);
        let error = decode_audio_delta(&encoded).expect_err("oversized audio delta decoded");
        assert!(error.message().contains("encoded realtime audio delta"));
    }
}
