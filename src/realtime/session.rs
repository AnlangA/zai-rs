//! [`RealtimeSession`] — an active realtime conversation over WebSocket.
//!
//! A session owns a background event-loop task that pumps client events onto
//! the socket and fans server events (and decoded audio) out to subscribers.
//! Callers drive it via command methods (`send_audio`, `send_text`, …) and
//! consume the two streams: [`RealtimeSession::events`] and
//! [`RealtimeSession::audio_stream`].

use std::{pin::Pin, sync::Arc};

use bytes::Bytes;
use futures::{Stream, StreamExt};
use tokio::{
    sync::{broadcast, mpsc},
    task::JoinHandle,
};
use tokio_stream::wrappers::BroadcastStream;
use tracing::{debug, warn};

use super::{
    audio::{OutputAudioFormat, decode_base64, encode_wav_pcm_base64},
    client::AuthMode,
    events::{ClientEvent, ServerEvent},
    jwt,
    protocol::{ChatMode, RealtimeTool, SessionConfig, TurnDetectionType},
    transport::{RealtimeTransport, WsMessage},
};
use crate::{ZaiResult, client::error::RealtimeErrorKind};

/// Commands queued onto the event-loop task. `ClientEvent` is boxed to keep
/// the enum small (the event payload is ~224 bytes vs. a tag for `Close`).
enum Command {
    /// Serialize + send this client event.
    ClientEvent(Box<ClientEvent>),
    /// Tear the session down.
    Close,
}

/// Builder for an [`RealtimeSession`].
///
/// Produced by [`super::client::RealtimeClient::session`]. Configure the
/// session defaults, then [`SessionBuilder::build`] opens the WebSocket and
/// sends the initial `session.update`.
pub struct SessionBuilder {
    api_key: Arc<String>,
    auth: AuthMode,
    realtime_url: String,
    model_name: String,
    session_config: SessionConfig,
}

impl SessionBuilder {
    pub(super) fn new(
        api_key: Arc<String>,
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

    /// Output audio format (defaults to PCM).
    pub fn output_audio_format(mut self, format: OutputAudioFormat) -> Self {
        self.session_config.output_audio_format = format;
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

    /// Register function tools.
    pub fn tools(mut self, tools: Vec<RealtimeTool>) -> Self {
        self.session_config.tools = tools;
        self
    }

    /// Override the entire session config.
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
            session_config,
        } = self;

        let jwt_ttl = match auth {
            AuthMode::Bearer => None,
            AuthMode::Jwt { ttl_seconds } => Some(ttl_seconds),
        };
        let authorization = jwt::authorization_header(&api_key, jwt_ttl)?;

        let mut transport =
            super::transport::TungsteniteTransport::connect(&realtime_url, &authorization).await?;

        // Initial session.update with the negotiated defaults.
        let init = ClientEvent::SessionUpdate {
            event_id: Some(new_event_id()),
            session: session_config,
        };
        transport.send(serde_json::to_string(&init)?).await?;
        debug!(model = %model_name, "Realtime session opened");

        let (cmd_tx, cmd_rx) = mpsc::channel::<Command>(64);
        let (events_tx, _) = broadcast::channel::<ServerEvent>(256);
        let (audio_tx, _) = broadcast::channel::<Bytes>(256);

        let join = tokio::spawn(run_loop(
            transport,
            cmd_rx,
            events_tx.clone(),
            audio_tx.clone(),
        ));

        Ok(RealtimeSession {
            cmd_tx,
            events_tx,
            audio_tx,
            model_name,
            join,
        })
    }
}

/// Background event-loop body: drains commands onto the socket and fans server
/// messages out to the broadcast channels. Generic over the transport so a mock
/// can be substituted in tests.
async fn run_loop<T: RealtimeTransport>(
    mut transport: T,
    mut cmd_rx: mpsc::Receiver<Command>,
    events_tx: broadcast::Sender<ServerEvent>,
    audio_tx: broadcast::Sender<Bytes>,
) {
    loop {
        tokio::select! {
            biased;
            cmd = cmd_rx.recv() => match cmd {
                Some(Command::ClientEvent(ev)) => {
                    let json = match serde_json::to_string(&ev) {
                        Ok(s) => s,
                        // Drop a malformed event but keep the session alive.
                        Err(_) => continue,
                    };
                    if transport.send(json).await.is_err() {
                        break;
                    }
                },
                Some(Command::Close) | None => {
                    let _ = transport.close().await;
                    debug!("Realtime session closed (client requested)");
                    break;
                },
            },
            msg = transport.recv() => match msg {
                Ok(Some(WsMessage::Text(text))) => match serde_json::from_str::<ServerEvent>(&text) {
                    Ok(ServerEvent::ResponseAudioDelta { delta, .. }) => {
                        if let Ok(bytes) = decode_base64(&delta) {
                            let _ = audio_tx.send(Bytes::from(bytes));
                        }
                    },
                    Ok(ServerEvent::Error { error }) => {
                        warn!(
                            code = ?error.code,
                            message = ?error.message,
                            "Realtime server error event"
                        );
                        let _ = events_tx.send(ServerEvent::Error { error });
                    },
                    Ok(event) => {
                        let _ = events_tx.send(event);
                    },
                    // Ignore unparseable/unknown frames; the session stays open.
                    Err(_) => {
                        warn!(
                            bytes = text.len(),
                            "Dropping unparseable realtime frame"
                        );
                    },
                },
                Ok(Some(WsMessage::Binary(_))) => {},
                Ok(None) => {
                    // peer closed cleanly
                    debug!("Realtime session closed (peer disconnected)");
                    break;
                },
                Err(e) => {
                    warn!(error = %e, "Realtime event loop terminated due to transport error");
                    break;
                },
            },
        }
    }
}

/// An active realtime session.
///
/// Cheap to share indirectly via the channels it owns; call
/// [`RealtimeSession::close`] to terminate the background task.
pub struct RealtimeSession {
    cmd_tx: mpsc::Sender<Command>,
    events_tx: broadcast::Sender<ServerEvent>,
    audio_tx: broadcast::Sender<Bytes>,
    model_name: String,
    join: JoinHandle<()>,
}

impl RealtimeSession {
    /// Send raw 16-bit LE mono PCM (16 kHz) audio; it is wrapped in a WAV
    /// header, base64-encoded, and uploaded via `input_audio_buffer.append`.
    pub async fn send_audio(&self, pcm: Bytes) -> ZaiResult<()> {
        let audio = encode_wav_pcm_base64(&pcm, 16_000);
        self.dispatch(ClientEvent::InputAudioBufferAppend {
            audio,
            client_timestamp: Some(now_ms()),
        })
        .await
    }

    /// Commit buffered audio for inference (client-VAD). Server-VAD commits
    /// automatically; calling this is harmless.
    pub async fn commit_audio(&self) -> ZaiResult<()> {
        self.dispatch(ClientEvent::InputAudioBufferCommit {
            client_timestamp: Some(now_ms()),
        })
        .await
    }

    /// Inject a user text message into the conversation history.
    pub async fn send_text(&self, text: impl Into<String>) -> ZaiResult<()> {
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
        self.dispatch(ClientEvent::ConversationItemCreate {
            event_id: Some(new_event_id()),
            item: super::protocol::RealtimeConversationItem::function_output(call_name, output),
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

    /// Stream of all server events (transcripts, response lifecycle, errors,
    /// heartbeats). Late subscribers miss events broadcast before they joined;
    /// subscribe before driving commands when ordering matters. Transient
    /// `Lagged` gaps (slow consumer) are dropped rather than erroring.
    pub fn events(&self) -> Pin<Box<dyn Stream<Item = ServerEvent> + Send + '_>> {
        let stream = BroadcastStream::new(self.events_tx.subscribe())
            .filter_map(|res| async move { res.ok() });
        Box::pin(stream)
    }

    /// Stream of decoded audio output chunks (PCM/MP3 bytes, per
    /// `output_audio_format`).
    pub fn audio_stream(&self) -> Pin<Box<dyn Stream<Item = Bytes> + Send + '_>> {
        let stream = BroadcastStream::new(self.audio_tx.subscribe())
            .filter_map(|res| async move { res.ok() });
        Box::pin(stream)
    }

    /// The model id this session was opened for (metadata; the protocol does
    /// not transmit it on the wire).
    pub fn model_name(&self) -> &str {
        &self.model_name
    }

    #[tracing::instrument(name = "realtime.dispatch", skip(self, event))]
    async fn dispatch(&self, event: ClientEvent) -> ZaiResult<()> {
        self.cmd_tx
            .send(Command::ClientEvent(Box::new(event)))
            .await
            .map_err(|_| RealtimeErrorKind::Closed.into())
    }

    /// Close the session and wait for the event loop to finish.
    pub async fn close(self) -> ZaiResult<()> {
        let _ = self.cmd_tx.send(Command::Close).await;
        // Dropping the last Sender closes the command channel; the event loop
        // observes `Command::Close` (sent above) and exits.
        let _ = self.join.await;
        Ok(())
    }
}

fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

fn new_event_id() -> String {
    format!("evt_{}", uuid::Uuid::new_v4().simple())
}
