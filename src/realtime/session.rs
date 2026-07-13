//! [`RealtimeSession`] — an active realtime conversation over WebSocket.
//!
//! A session owns a background event-loop task that pumps client events onto
//! the socket and fans server events (and decoded audio) out to subscribers.
//! Callers drive it via command methods (`send_audio`, `send_text`, …) and
//! consume the two streams: [`RealtimeSession::events`] and
//! [`RealtimeSession::audio_stream`].

use std::{
    pin::Pin,
    sync::{Arc, Mutex},
    time::Duration,
};

use bytes::Bytes;
use futures_util::{Stream, stream};
use tokio::{
    sync::{broadcast, mpsc, watch},
    task::JoinHandle,
};
use tracing::{debug, warn};

use super::{
    audio::{
        InputAudioFormat, OutputAudioFormat, decode_base64, encode_base64,
        encode_jpeg_frame_base64, encode_wav_pcm_base64,
    },
    client::AuthMode,
    events::{ClientEvent, ServerEvent},
    jwt,
    protocol::{
        ChatMode, GreetingConfig, InputAudioNoiseReduction, NoiseReductionType, RealtimeModality,
        RealtimeTool, RealtimeVoice, SessionConfig, TurnDetectionType,
    },
    transport::{RealtimeTransport, WsMessage},
};
use crate::{
    ZaiResult,
    client::{
        error::RealtimeErrorKind,
        secret::ApiSecret,
        transport::limits::{REALTIME_AUDIO_FRAME_MAX, WS_MESSAGE_MAX},
    },
};

mod validation;

use validation::validate_session_config;

/// The server documents an application heartbeat approximately every 30
/// seconds. Three missed heartbeats is treated as a dead/half-open session.
const INBOUND_IDLE_TIMEOUT: Duration = Duration::from_secs(90);
/// Frozen realtime contract: each per-session queue is deliberately small so
/// backpressure or consumer lag is detected before large audio/event backlogs
/// accumulate in memory.
const SESSION_CHANNEL_CAPACITY: usize = 8;

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
}

impl SessionBuilder {
    pub(super) fn new(
        api_key: Arc<ApiSecret>,
        auth: AuthMode,
        realtime_url: String,
        model_name: String,
    ) -> Self {
        Self {
            api_key,
            auth,
            realtime_url,
            model_name,
            session_config: SessionConfig::default(),
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
    /// and replaces `config.model` during [`Self::build`].
    pub fn session_config(mut self, config: SessionConfig) -> Self {
        self.session_config = config;
        self
    }

    /// Open the WebSocket, send `session.update`, and spawn the event loop.
    #[tracing::instrument(name = "realtime.session.build", skip_all, fields(model = %self.model_name))]
    pub async fn build(self) -> ZaiResult<RealtimeSession> {
        let Self {
            api_key,
            auth,
            realtime_url,
            model_name,
            mut session_config,
        } = self;

        // The selected type-safe model is part of the session.update wire
        // contract. It takes precedence over an arbitrary value supplied via
        // `session_config` so the marker-trait guarantee cannot be bypassed.
        session_config.model = Some(model_name.clone());
        validate_session_config(&session_config)?;
        let input_audio_format = session_config.input_audio_format;
        let init = ClientEvent::SessionUpdate {
            event_id: Some(new_event_id()),
            session: session_config,
        };
        // Serialize and enforce the message limit before opening a socket so a
        // locally invalid configuration cannot cause network side effects.
        let init = serialize_event(&init)?;

        let jwt_ttl = match auth {
            AuthMode::Bearer => None,
            AuthMode::Jwt { ttl_seconds } => Some(ttl_seconds),
        };
        let authorization = jwt::authorization_header(api_key.expose(), jwt_ttl)?;

        let mut transport =
            super::transport::TungsteniteTransport::connect(&realtime_url, &authorization).await?;

        if let Err(error) = transport.send(init).await {
            let _ = transport.close().await;
            return Err(error);
        }
        debug!(model = %model_name, "Realtime session opened");

        let (cmd_tx, cmd_rx) = mpsc::channel::<String>(SESSION_CHANNEL_CAPACITY);
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        let (events_tx, _) = broadcast::channel::<ServerEvent>(SESSION_CHANNEL_CAPACITY);
        let (audio_tx, _) = broadcast::channel::<RealtimeAudioChunk>(SESSION_CHANNEL_CAPACITY);
        // Subscribe before the event loop starts so session-created events and
        // greeting audio cannot race ahead of the caller's first subscription.
        let initial_events_rx = events_tx.subscribe();
        let initial_audio_rx = audio_tx.subscribe();

        let (completion_tx, completion_rx) = watch::channel(None);
        let loop_events_tx = events_tx.clone();
        let loop_audio_tx = audio_tx.clone();
        let join = tokio::spawn(async move {
            let result = run_loop(
                transport,
                cmd_rx,
                shutdown_rx,
                loop_events_tx,
                loop_audio_tx,
            )
            .await;
            completion_tx.send_replace(Some(result.clone()));
            result
        });

        Ok(RealtimeSession {
            cmd_tx,
            shutdown_tx,
            events_tx,
            audio_tx,
            initial_events_rx: Mutex::new(Some(initial_events_rx)),
            initial_audio_rx: Mutex::new(Some(initial_audio_rx)),
            completion_rx,
            model_name,
            input_audio_format,
            join,
        })
    }
}

/// Background event-loop body: drains commands onto the socket and fans server
/// messages out to the broadcast channels. Generic over the transport so a mock
/// can be substituted in tests.
async fn run_loop<T: RealtimeTransport>(
    mut transport: T,
    mut cmd_rx: mpsc::Receiver<String>,
    mut shutdown_rx: watch::Receiver<bool>,
    events_tx: broadcast::Sender<ServerEvent>,
    audio_tx: broadcast::Sender<RealtimeAudioChunk>,
) -> ZaiResult<()> {
    let idle_deadline = tokio::time::sleep(INBOUND_IDLE_TIMEOUT);
    tokio::pin!(idle_deadline);

    loop {
        tokio::select! {
            biased;
            changed = shutdown_rx.changed() => {
                if changed.is_err() || *shutdown_rx.borrow() {
                    debug!("Realtime session closed (client requested)");
                    return transport.close().await;
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
                        .reset(tokio::time::Instant::now() + INBOUND_IDLE_TIMEOUT);
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
                            let _ = transport.close().await;
                            return Err(error);
                        },
                    }

                    match cmd_rx.try_recv() {
                        Ok(command) => {
                            if !handle_outbound(&mut transport, Some(command)).await? {
                                return Ok(());
                            }
                        },
                        Err(mpsc::error::TryRecvError::Disconnected) => {
                            handle_outbound(&mut transport, None).await?;
                            return Ok(());
                        },
                        Err(mpsc::error::TryRecvError::Empty) => {},
                    }
                },
                Ok(Some(WsMessage::Binary(bytes))) => {
                    warn!(bytes = bytes.len(), "Closing session after unexpected realtime binary frame");
                    let _ = transport.close().await;
                    return Err(protocol_error(
                        "unexpected binary frame in realtime JSON protocol",
                    ));
                },
                Ok(None) => {
                    debug!("Realtime session closed (peer disconnected)");
                    return Ok(());
                },
                Err(error) => {
                    // Avoid logging the error source: handshake/transport errors
                    // can contain endpoint details or server-provided text.
                    warn!("Realtime event loop terminated due to transport error");
                    let _ = transport.close().await;
                    return Err(error);
                },
            },
            _ = &mut idle_deadline => {
                warn!(
                    timeout_seconds = INBOUND_IDLE_TIMEOUT.as_secs(),
                    "Realtime session timed out waiting for inbound traffic"
                );
                let _ = transport.close().await;
                return Err(RealtimeErrorKind::Timeout {
                    operation: "Realtime inbound heartbeat",
                }
                .into());
            },
            cmd = cmd_rx.recv() => {
                if !handle_outbound(&mut transport, cmd).await? {
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
    let event = serde_json::from_str::<ServerEvent>(text)
        .map_err(|_| protocol_error("malformed realtime server event"))?;
    match event {
        ServerEvent::ResponseAudioDelta {
            response_id,
            item_id,
            output_index,
            content_index,
            delta,
        } => {
            let bytes = decode_base64(&delta)?;
            if bytes.len() as u64 > REALTIME_AUDIO_FRAME_MAX {
                return Err(protocol_error(format!(
                    "realtime audio delta exceeds {REALTIME_AUDIO_FRAME_MAX} bytes"
                )));
            }
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

async fn handle_outbound<T: RealtimeTransport>(
    transport: &mut T,
    message: Option<String>,
) -> ZaiResult<bool> {
    match message {
        Some(json) => {
            if let Err(error) = transport.send(json).await {
                let _ = transport.close().await;
                return Err(error);
            }
            Ok(true)
        },
        None => {
            debug!("Realtime session closed (client requested)");
            transport.close().await?;
            Ok(false)
        },
    }
}

/// An active realtime session.
///
/// Cheap to share indirectly via the channels it owns; call
/// [`RealtimeSession::close`] to terminate the background task.
pub struct RealtimeSession {
    cmd_tx: mpsc::Sender<String>,
    shutdown_tx: watch::Sender<bool>,
    events_tx: broadcast::Sender<ServerEvent>,
    audio_tx: broadcast::Sender<RealtimeAudioChunk>,
    initial_events_rx: Mutex<Option<broadcast::Receiver<ServerEvent>>>,
    initial_audio_rx: Mutex<Option<broadcast::Receiver<RealtimeAudioChunk>>>,
    completion_rx: watch::Receiver<Option<ZaiResult<()>>>,
    model_name: String,
    input_audio_format: InputAudioFormat,
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
        let audio = match self.input_audio_format {
            InputAudioFormat::Wav => encode_wav_pcm_base64(&pcm, 16_000)?,
            InputAudioFormat::Pcm16 | InputAudioFormat::Pcm24 => encode_base64(&pcm),
        };
        self.dispatch(ClientEvent::InputAudioBufferAppend {
            audio,
            client_timestamp: Some(now_ms()),
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
        self.dispatch(ClientEvent::InputAudioBufferAppendVideoFrame {
            video_frame: encode_jpeg_frame_base64(&jpeg),
            client_timestamp: Some(now_ms()),
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

    #[tracing::instrument(name = "realtime.dispatch", skip(self, event))]
    async fn dispatch(&self, event: ClientEvent) -> ZaiResult<()> {
        let message = serialize_event(&event)?;
        self.cmd_tx
            .send(message)
            .await
            .map_err(|_| RealtimeErrorKind::Closed.into())
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
            tokio::time::sleep(INBOUND_IDLE_TIMEOUT).await;
            Ok(Some(WsMessage::Text(r#"{"type":"heartbeat"}"#.into())))
        }

        async fn close(&mut self) -> crate::ZaiResult<()> {
            self.closed.store(true, Ordering::Relaxed);
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

    fn spawn_test_loop<T>(
        transport: T,
        cmd_rx: mpsc::Receiver<String>,
        events_tx: broadcast::Sender<ServerEvent>,
        audio_tx: broadcast::Sender<RealtimeAudioChunk>,
    ) -> (watch::Sender<bool>, JoinHandle<ZaiResult<()>>)
    where
        T: RealtimeTransport + 'static,
    {
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        let join = tokio::spawn(run_loop(
            transport,
            cmd_rx,
            shutdown_rx,
            events_tx,
            audio_tx,
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
        tokio::time::advance(INBOUND_IDLE_TIMEOUT).await;
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
        tokio::time::advance(INBOUND_IDLE_TIMEOUT).await;
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
    fn session_queues_match_frozen_contract_capacity() {
        assert_eq!(SESSION_CHANNEL_CAPACITY, 8);
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
        let (events_tx, initial_rx) = broadcast::channel(SESSION_CHANNEL_CAPACITY);
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
