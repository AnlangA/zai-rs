//! [`RealtimeSession`] — an active realtime conversation over WebSocket.
//!
//! A session owns a background event-loop task that pumps client events onto
//! the socket and fans server events (and decoded audio) out to subscribers.
//! Callers drive it via command methods (`send_audio`, `send_text`, …) and
//! consume the two streams: [`RealtimeSession::events`] and
//! [`RealtimeSession::audio_stream`].

use std::{pin::Pin, sync::Arc};

use bytes::Bytes;
use futures_util::{Stream, StreamExt};
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
                        Err(e) => {
                            warn!(
                                error = %e,
                                "Dropping realtime client event that failed to serialize"
                            );
                            continue;
                        }
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
        let stream = BroadcastStream::new(self.events_tx.subscribe()).filter_map(|res| {
            async move {
                match res {
                    Ok(event) => Some(event),
                    // A slow consumer fell behind the broadcast buffer. Surface
                    // the gap so it is observable (the stream still continues
                    // from the live tail rather than terminating).
                    Err(_) => {
                        warn!("Realtime events consumer lagged; some events were dropped");
                        None
                    },
                }
            }
        });
        Box::pin(stream)
    }

    /// Stream of decoded audio output chunks (PCM/MP3 bytes, per
    /// `output_audio_format`).
    pub fn audio_stream(&self) -> Pin<Box<dyn Stream<Item = Bytes> + Send + '_>> {
        let stream = BroadcastStream::new(self.audio_tx.subscribe()).filter_map(|res| async move {
            match res {
                Ok(bytes) => Some(bytes),
                Err(_) => {
                    warn!("Realtime audio consumer lagged; some audio chunks were dropped");
                    None
                },
            }
        });
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

    /// Signal the background task to close without awaiting it.
    ///
    /// Use when the session is shared (e.g. behind an `Arc`), driven from a
    /// `tokio::select!`, or closed reactively on a shutdown signal. This only
    /// enqueues `Command::Close`; the background loop observes it and exits.
    /// For deterministic, awaited teardown use [`RealtimeSession::close`].
    pub async fn request_close(&self) -> ZaiResult<()> {
        self.cmd_tx
            .send(Command::Close)
            .await
            .map_err(|_| RealtimeErrorKind::Closed.into())
    }

    /// Close the session and wait for the event loop to finish.
    pub async fn close(self) -> ZaiResult<()> {
        // Best-effort: enqueue Close even if the channel already tore down.
        let _ = self.cmd_tx.send(Command::Close).await;
        // Dropping the last Sender closes the command channel; the event loop
        // observes `Command::Close` (sent above) and exits. Surface a JoinError
        // (a panicked event loop / runtime cancellation) via a warning instead
        // of silently discarding it.
        if let Err(join_err) = self.join.await {
            warn!(error = %join_err, "realtime event loop ended abnormally");
        }
        Ok(())
    }
}

fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

fn new_event_id() -> String {
    format!("evt_{}", uuid::Uuid::new_v4().simple())
}

#[cfg(test)]
mod tests {
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
        let (cmd_tx, cmd_rx) = mpsc::channel::<Command>(8);
        let (events_tx, _) = broadcast::channel::<ServerEvent>(16);
        let (audio_tx, _) = broadcast::channel::<Bytes>(16);
        let join = tokio::spawn(run_loop(transport, cmd_rx, events_tx, audio_tx));

        // Drop the last sender → `cmd_rx.recv()` returns `None` → the loop
        // calls `transport.close()` and exits.
        drop(cmd_tx);

        let joined = tokio::time::timeout(std::time::Duration::from_secs(2), join)
            .await
            .expect("run_loop did not terminate after the command sender dropped");
        joined.expect("run_loop task panicked");
        assert!(
            *closed.lock().unwrap(),
            "transport.close() was not invoked on teardown"
        );
    }
}
