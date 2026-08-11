//! Realtime transport abstraction.
//!
//! [`RealtimeTransport`] is the realtime analogue of the HTTP client trait: it
//! isolates the WebSocket transport behind a small async interface so the
//! session/event-loop logic is testable and transport-agnostic. The default
//! implementation, [`TungsteniteTransport`], uses `tokio-tungstenite` over TLS
//! (rustls).

use async_trait::async_trait;
use bytes::Bytes;
use futures_util::{Sink, SinkExt, StreamExt, stream::SplitStream};
use http::{HeaderValue, header::AUTHORIZATION};
use std::{sync::Arc, time::Duration};
use tokio::{
    sync::{OwnedSemaphorePermit, Semaphore, mpsc, oneshot, watch},
    task::JoinHandle,
    time::Instant,
};
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream, connect_async_with_config,
    tungstenite::{
        Error as WebSocketError,
        client::IntoClientRequest,
        protocol::{
            Message, WebSocketConfig,
            frame::{
                Frame,
                coding::{Data, OpCode},
            },
        },
    },
};
use tracing::{debug, warn};

use super::config::RealtimeTransportConfig;

use crate::{
    ZaiResult,
    client::{error::RealtimeErrorKind, transport::limits::WS_MESSAGE_MAX},
};

const WRITER_BUFFER_BYTES_MAX: usize = WS_MESSAGE_MAX as usize;
const TUNGSTENITE_WRITE_BUFFER_TARGET_MAX: usize = 128 * 1024;

type WebSocket = WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;
type WebSocketReader = SplitStream<WebSocket>;

#[derive(Debug, Clone, Copy)]
struct WriterPolicy {
    write_timeout: Duration,
    pong_timeout: Duration,
    data_frame_stall_timeout: Duration,
    close_timeout: Duration,
    writer_join_timeout: Duration,
    confirmed_write_timeout: Duration,
    max_frame_size: usize,
}

impl WriterPolicy {
    fn from_transport_config(config: &RealtimeTransportConfig) -> Self {
        Self {
            write_timeout: config.write_timeout(),
            pong_timeout: config.pong_timeout(),
            data_frame_stall_timeout: config.data_frame_stall_timeout(),
            close_timeout: config.close_timeout(),
            writer_join_timeout: config.writer_join_timeout(),
            confirmed_write_timeout: config.confirmed_write_timeout(),
            max_frame_size: config.max_frame_bytes(),
        }
    }
}

struct QueuedFrame {
    text: String,
    _byte_budget: OwnedSemaphorePermit,
    _slot_budget: Option<OwnedSemaphorePermit>,
    completion: Option<oneshot::Sender<ZaiResult<()>>>,
}

/// End-to-end admission owned by a built-in session command.
///
/// The byte permit is present for every session command. Built-in sessions add
/// an end-to-end count permit; injected sessions deliberately leave it absent
/// because their public capacity denotes queued commands, excluding the one
/// third-party `send` currently in flight. Any present permits move into the
/// writer frame and remain held until the sink completes or discards it. This
/// prevents the private writer from performing a second, fallible admission
/// after the public session API has already accepted the command.
pub(crate) struct SessionFrameBudget {
    byte_budget: OwnedSemaphorePermit,
    slot_budget: Option<OwnedSemaphorePermit>,
}

impl SessionFrameBudget {
    pub(crate) fn new(
        byte_budget: OwnedSemaphorePermit,
        slot_budget: Option<OwnedSemaphorePermit>,
    ) -> Self {
        Self {
            byte_budget,
            slot_budget,
        }
    }
}

#[derive(Clone)]
struct QueuedControlFlush {
    operation: &'static str,
    deadline: Instant,
}

#[derive(Clone, Copy)]
enum WriterPreference {
    Control,
    Data,
}

enum WriterWork {
    Shutdown,
    Continue,
    ControlClosed,
    DataClosed,
    Control(QueuedControlFlush),
    Data(QueuedFrame),
}

/// A single inbound WebSocket message of interest to realtime callers.
#[derive(Debug, Clone)]
pub enum WsMessage {
    /// A text frame (realtime JSON events).
    Text(String),
    /// A binary frame.
    Binary(Bytes),
}

/// Async WebSocket transport used by [`super::session::RealtimeSession`].
///
/// Mirrors the crate's trait-driven design: the session depends on this trait,
/// not on any concrete client, so a mock transport can be substituted in tests.
#[async_trait]
pub trait RealtimeTransport: Send {
    /// Send a text frame.
    ///
    /// Implementations used with
    /// [`SessionBuilder::build_with_transport`](super::session::SessionBuilder::build_with_transport)
    /// should make this a cancellation-safe, bounded admission operation so a
    /// slow socket cannot stop the session from receiving events. The built-in
    /// public transport instead resolves only after the frame is written;
    /// session internals wrap it in a private bounded adapter.
    async fn send(&mut self, msg: String) -> ZaiResult<()>;
    /// Send the initial `session.update` and confirm that it has been written.
    ///
    /// Confirmation means the complete message has reached the underlying
    /// transport sink; it does not wait for a server `session.updated` event.
    /// The default is source-compatible for transports whose [`send`](Self::send)
    /// already has confirmed-write semantics. Buffered implementations must
    /// override this method so session construction cannot return while the
    /// initial update is merely queued. An injected session may cancel this
    /// future at its hard deadline and then call [`close`](Self::close), so the
    /// operation must leave the transport safe to close when cancelled.
    async fn send_confirmed(&mut self, msg: String) -> ZaiResult<()> {
        self.send(msg).await
    }
    /// Receive the next meaningful message, or `None` when the peer closed.
    ///
    /// This future must be cancellation-safe: the session event loop may stop
    /// polling it whenever an outbound command or shutdown signal wins.
    async fn recv(&mut self) -> ZaiResult<Option<WsMessage>>;
    /// Gracefully close the connection.
    ///
    /// Injected sessions may cancel this future at the policy's outer close
    /// deadline. Implementations must remain safe to drop after cancellation
    /// and must not require a later poll to preserve memory or resource safety.
    async fn close(&mut self) -> ZaiResult<()>;
}

/// `tokio-tungstenite`-backed WebSocket transport over TLS (rustls).
pub struct TungsteniteTransport {
    reader: WebSocketReader,
    writer_tx: mpsc::Sender<QueuedFrame>,
    control_tx: watch::Sender<Option<QueuedControlFlush>>,
    writer_budget: Arc<Semaphore>,
    shutdown_tx: watch::Sender<bool>,
    writer_status: watch::Receiver<Option<ZaiResult<()>>>,
    writer_join: Option<JoinHandle<ZaiResult<()>>>,
    writer_close_result: Option<ZaiResult<()>>,
    writer_policy: WriterPolicy,
    transport_config: RealtimeTransportConfig,
}

/// Session-only adapter whose `send` admits frames to the bounded background
/// writer. The public transport retains confirmed-send semantics for callers
/// that use [`RealtimeTransport`] directly.
pub(crate) struct BufferedTungsteniteTransport(TungsteniteTransport);

impl TungsteniteTransport {
    /// Open one WebSocket connection attempt to `url` with the given
    /// `Authorization` header value.
    ///
    /// This direct entry point does not perform connection retries. Use a
    /// built-in [`SessionBuilder`](super::SessionBuilder) when the SDK should
    /// apply bounded pre-`session.update` connection recovery.
    #[tracing::instrument(name = "realtime.connect", skip_all)]
    pub async fn connect(url: &str, authorization: &str) -> ZaiResult<Self> {
        Self::connect_with_config(url, authorization, RealtimeTransportConfig::default()).await
    }

    /// Open a WebSocket with an explicit, owned transport policy.
    ///
    /// The policy is validated before any network operation and retained as
    /// the effective configuration returned by [`Self::transport_config`]. A
    /// direct transport performs exactly one attempt and consumes
    /// connect/write/Pong/close, writer-queue, and frame settings. It retains
    /// but does not consume `max_connect_attempts`; session-owned connection
    /// recovery, admission, and event/audio buffer settings take effect only
    /// when the same policy is used by a
    /// [`SessionBuilder`](super::SessionBuilder).
    #[tracing::instrument(name = "realtime.connect", skip_all)]
    pub async fn connect_with_config(
        url: &str,
        authorization: &str,
        transport_config: RealtimeTransportConfig,
    ) -> ZaiResult<Self> {
        transport_config.validate()?;
        let mut req = url.into_client_request()?;
        let mut auth_value = HeaderValue::from_str(authorization).map_err(|e| {
            RealtimeErrorKind::Protocol(format!("invalid Authorization value: {e}"))
        })?;
        auth_value.set_sensitive(true);
        req.headers_mut().insert(AUTHORIZATION, auth_value);

        let writer_policy = WriterPolicy::from_transport_config(&transport_config);
        let config = websocket_config(writer_policy.max_frame_size)?;
        let (stream, _response) = tokio::time::timeout(
            transport_config.connect_timeout(),
            connect_async_with_config(req, Some(config), false),
        )
        .await
        .map_err(|_| RealtimeErrorKind::Timeout {
            operation: "WebSocket connect",
        })??;
        let (writer, reader) = stream.split();
        let (writer_tx, writer_rx) = mpsc::channel(transport_config.writer_queue_capacity());
        let (control_tx, control_rx) = watch::channel(None);
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        let (writer_status_tx, writer_status) = watch::channel(None);
        let writer_join = tokio::spawn(writer_task(
            writer,
            writer_rx,
            control_rx,
            shutdown_rx,
            writer_policy,
            writer_status_tx,
        ));
        debug!("Realtime WebSocket connected");
        Ok(Self {
            reader,
            writer_tx,
            control_tx,
            writer_budget: Arc::new(Semaphore::new(WRITER_BUFFER_BYTES_MAX)),
            shutdown_tx,
            writer_status,
            writer_join: Some(writer_join),
            writer_close_result: None,
            writer_policy,
            transport_config,
        })
    }

    /// The validated policy retained by this connected transport.
    ///
    /// Wire-side settings are active here; the connection-attempt limit and
    /// session-only queue/broadcast fields are retained for inspection but
    /// require a `SessionBuilder` to take effect.
    pub fn transport_config(&self) -> &RealtimeTransportConfig {
        &self.transport_config
    }

    /// Send a frame and wait until the writer has completed it. This is used
    /// for the initial `session.update`; regular session writes use the
    /// transport's bounded asynchronous writer.
    async fn send_confirmed_frame(&mut self, msg: String) -> ZaiResult<()> {
        let (completion, completed) = oneshot::channel();
        self.enqueue_text(msg, Some(completion))?;
        tokio::time::timeout(self.writer_policy.confirmed_write_timeout, completed)
            .await
            .map_err(|_| RealtimeErrorKind::Timeout {
                operation: "WebSocket send",
            })?
            .map_err(|_| self.writer_error_or_closed())?
    }

    pub(crate) fn into_buffered(self) -> BufferedTungsteniteTransport {
        BufferedTungsteniteTransport(self)
    }

    fn enqueue_text(
        &self,
        msg: String,
        completion: Option<oneshot::Sender<ZaiResult<()>>>,
    ) -> ZaiResult<()> {
        validate_outbound_message(&msg)?;
        let permits = u32::try_from(msg.len())
            .map_err(|_| RealtimeErrorKind::Protocol("realtime message length overflow".into()))?;
        let budget = Arc::clone(&self.writer_budget)
            .try_acquire_many_owned(permits)
            .map_err(|_| {
                RealtimeErrorKind::Protocol(format!(
                    "realtime outbound writer backlog exceeds {WRITER_BUFFER_BYTES_MAX} bytes"
                ))
            })?;
        self.enqueue_prebudgeted_text(msg, budget, None, completion)
    }

    fn enqueue_prebudgeted_text(
        &self,
        msg: String,
        byte_budget: OwnedSemaphorePermit,
        slot_budget: Option<OwnedSemaphorePermit>,
        completion: Option<oneshot::Sender<ZaiResult<()>>>,
    ) -> ZaiResult<()> {
        let frame = QueuedFrame {
            text: msg,
            _byte_budget: byte_budget,
            _slot_budget: slot_budget,
            completion,
        };
        self.writer_tx.try_send(frame).map_err(|error| match error {
            mpsc::error::TrySendError::Full(_) => RealtimeErrorKind::Protocol(format!(
                "realtime outbound writer queue exceeds {} messages",
                self.transport_config.writer_queue_capacity()
            ))
            .into(),
            mpsc::error::TrySendError::Closed(_) => self.writer_error_or_closed(),
        })
    }

    fn writer_error_or_closed(&self) -> crate::ZaiError {
        match self.writer_status.borrow().clone() {
            Some(Err(error)) => error,
            _ => RealtimeErrorKind::Closed.into(),
        }
    }
}

fn websocket_config(frame_max: usize) -> ZaiResult<WebSocketConfig> {
    let write_buffer_size = TUNGSTENITE_WRITE_BUFFER_TARGET_MAX.min(frame_max);
    let max_write_buffer_size = frame_max
        .checked_mul(2)
        .ok_or_else(|| RealtimeErrorKind::Protocol("realtime frame limit overflow".into()))?;
    if frame_max == 0 || max_write_buffer_size <= write_buffer_size {
        return Err(RealtimeErrorKind::Protocol(
            "realtime frame limit cannot produce a valid WebSocket write buffer".into(),
        )
        .into());
    }
    Ok(WebSocketConfig::default()
        .write_buffer_size(write_buffer_size)
        .max_message_size(Some(WS_MESSAGE_MAX as usize))
        .max_frame_size(Some(frame_max))
        .max_write_buffer_size(max_write_buffer_size))
}

async fn writer_task<S>(
    writer: S,
    mut writer_rx: mpsc::Receiver<QueuedFrame>,
    control_rx: watch::Receiver<Option<QueuedControlFlush>>,
    shutdown_rx: watch::Receiver<bool>,
    policy: WriterPolicy,
    writer_status_tx: watch::Sender<Option<ZaiResult<()>>>,
) -> ZaiResult<()>
where
    S: Sink<Message, Error = WebSocketError> + Unpin,
{
    // Keep ownership of the receiver outside the core loop. Once the loop has
    // a concrete sink/timeout result, publish it before closing the channel so
    // every producer that observes `TrySendError::Closed` can recover the same
    // terminal cause from `writer_status`.
    let result = writer_loop(writer, &mut writer_rx, control_rx, shutdown_rx, policy).await;
    writer_status_tx.send_replace(Some(result.clone()));
    writer_rx.close();
    result
}

async fn writer_loop<S>(
    mut writer: S,
    writer_rx: &mut mpsc::Receiver<QueuedFrame>,
    mut control_rx: watch::Receiver<Option<QueuedControlFlush>>,
    mut shutdown_rx: watch::Receiver<bool>,
    policy: WriterPolicy,
) -> ZaiResult<()>
where
    S: Sink<Message, Error = WebSocketError> + Unpin,
{
    let mut preference = WriterPreference::Control;
    let mut control_open = true;
    let mut data_open = true;
    let result = loop {
        // Control flushes normally win so Pong latency remains bounded. After
        // one control flush, prefer an already-ready application message for
        // one turn. This prevents a self-replenishing Ping stream from keeping
        // `control_rx.changed()` permanently ready and starving writer data.
        // Shutdown remains the first branch in both states.
        let work = match preference {
            WriterPreference::Control => tokio::select! {
                biased;
                changed = shutdown_rx.changed() => {
                    if changed.is_err() || *shutdown_rx.borrow() {
                        WriterWork::Shutdown
                    } else {
                        WriterWork::Continue
                    }
                },
                changed = control_rx.changed(), if control_open => {
                    if changed.is_err() {
                        WriterWork::ControlClosed
                    } else {
                        match control_rx.borrow_and_update().clone() {
                            Some(control) => WriterWork::Control(control),
                            None => WriterWork::Continue,
                        }
                    }
                },
                frame = writer_rx.recv(), if data_open => match frame {
                    Some(frame) => WriterWork::Data(frame),
                    None => WriterWork::DataClosed,
                },
            },
            WriterPreference::Data => tokio::select! {
                biased;
                changed = shutdown_rx.changed() => {
                    if changed.is_err() || *shutdown_rx.borrow() {
                        WriterWork::Shutdown
                    } else {
                        WriterWork::Continue
                    }
                },
                frame = writer_rx.recv(), if data_open => match frame {
                    Some(frame) => WriterWork::Data(frame),
                    None => WriterWork::DataClosed,
                },
                changed = control_rx.changed(), if control_open => {
                    if changed.is_err() {
                        WriterWork::ControlClosed
                    } else {
                        match control_rx.borrow_and_update().clone() {
                            Some(control) => WriterWork::Control(control),
                            None => WriterWork::Continue,
                        }
                    }
                },
            },
        };

        match work {
            WriterWork::Shutdown => break Ok(()),
            WriterWork::Continue => {},
            WriterWork::ControlClosed => {
                control_open = false;
                if !data_open {
                    break Ok(());
                }
                preference = WriterPreference::Data;
            },
            WriterWork::DataClosed => {
                data_open = false;
                if !control_open {
                    break Ok(());
                }
                preference = WriterPreference::Control;
            },
            WriterWork::Control(control) => {
                match flush_automatic_pong(&mut writer, control, &mut shutdown_rx).await {
                    Ok(true) => {
                        preference = WriterPreference::Data;
                        // A synchronously-ready sink plus a feedback Ping can
                        // otherwise keep this task running without ever giving
                        // a producer the chance to make the data lane ready.
                        tokio::task::yield_now().await;
                    },
                    Ok(false) => break Ok(()),
                    Err(error) => break Err(error),
                }
            },
            WriterWork::Data(frame) => {
                let QueuedFrame {
                    text,
                    _byte_budget: byte_budget,
                    _slot_budget: slot_budget,
                    completion,
                } = frame;
                let sent = write_text_message(
                    &mut writer,
                    text,
                    &mut control_rx,
                    &mut shutdown_rx,
                    policy,
                )
                .await;
                drop(byte_budget);
                drop(slot_budget);
                if let Some(completion) = completion {
                    let completed = match &sent {
                        Ok(true) => Ok(()),
                        Ok(false) => Err(RealtimeErrorKind::Closed.into()),
                        Err(error) => Err(error.clone()),
                    };
                    let _ = completion.send(completed);
                }
                match sent {
                    Ok(true) => preference = WriterPreference::Control,
                    Ok(false) => break Ok(()),
                    Err(error) => break Err(error),
                }
            },
        }
    };

    // A timed-out/failed sink may retain a partially-written frame. Returning
    // immediately drops it rather than reusing corrupt state, and makes the
    // writer failure observable within the frame/Pong deadline.
    result?;
    tokio::time::timeout(policy.close_timeout, writer.close())
        .await
        .map_err(|_| RealtimeErrorKind::Timeout {
            operation: "WebSocket close",
        })?
        .map_err(|source| {
            warn!("WebSocket close failed");
            RealtimeErrorKind::WebSocket { source }.into()
        })
}

async fn join_writer_task(
    writer_join: &mut Option<JoinHandle<ZaiResult<()>>>,
    timeout: Duration,
) -> ZaiResult<()> {
    let Some(join) = writer_join.as_mut() else {
        return Ok(());
    };
    let result = match tokio::time::timeout(timeout, &mut *join).await {
        Ok(Ok(result)) => result,
        Ok(Err(join_error)) => Err(RealtimeErrorKind::Protocol(format!(
            "WebSocket writer task failed: {join_error}"
        ))
        .into()),
        Err(_) => {
            join.abort();
            Err(RealtimeErrorKind::Timeout {
                operation: "WebSocket close",
            }
            .into())
        },
    };
    writer_join.take();
    result
}

fn fragment_text(text: String, frame_max: usize) -> impl Iterator<Item = Message> {
    let payload = Bytes::from(text);
    let mut offset = 0;
    let mut emitted_empty = false;
    std::iter::from_fn(move || {
        if payload.is_empty() {
            if emitted_empty {
                return None;
            }
            emitted_empty = true;
            return Some(Message::Frame(Frame::message(
                Bytes::new(),
                OpCode::Data(Data::Text),
                true,
            )));
        }
        if offset == payload.len() {
            return None;
        }

        debug_assert!(frame_max > 0, "validated realtime frame limit is zero");
        let start = offset;
        let end = offset.saturating_add(frame_max).min(payload.len());
        offset = end;
        let opcode = if start == 0 {
            OpCode::Data(Data::Text)
        } else {
            OpCode::Data(Data::Continue)
        };
        Some(Message::Frame(Frame::message(
            payload.slice(start..end),
            opcode,
            end == payload.len(),
        )))
    })
}

async fn write_text_message<S>(
    writer: &mut S,
    text: String,
    control_rx: &mut watch::Receiver<Option<QueuedControlFlush>>,
    shutdown_rx: &mut watch::Receiver<bool>,
    policy: WriterPolicy,
) -> ZaiResult<bool>
where
    S: Sink<Message, Error = WebSocketError> + Unpin,
{
    let message_deadline = Instant::now() + policy.write_timeout;
    for fragment in fragment_text(text, policy.max_frame_size) {
        if control_rx.has_changed().unwrap_or(false) {
            let control = { control_rx.borrow_and_update().clone() };
            if let Some(control) = control
                && !flush_automatic_pong(writer, control, shutdown_rx).await?
            {
                return Ok(false);
            }
        }

        let remaining = message_deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(RealtimeErrorKind::Timeout {
                operation: "WebSocket send",
            }
            .into());
        }
        let frame_stall_timeout = remaining.min(policy.data_frame_stall_timeout);
        if !write_frame(
            writer,
            fragment,
            shutdown_rx,
            frame_stall_timeout,
            "WebSocket send",
        )
        .await?
        {
            return Ok(false);
        }
        // Give the reader task a chance to request a Pong flush before the next
        // continuation frame is selected.
        tokio::task::yield_now().await;
    }
    Ok(true)
}

async fn flush_automatic_pong<S>(
    writer: &mut S,
    control: QueuedControlFlush,
    shutdown_rx: &mut watch::Receiver<bool>,
) -> ZaiResult<bool>
where
    S: Sink<Message, Error = WebSocketError> + Unpin,
{
    let remaining = control.deadline.saturating_duration_since(Instant::now());
    if remaining.is_zero() {
        return Err(RealtimeErrorKind::Timeout {
            operation: control.operation,
        }
        .into());
    }
    flush_writer(writer, shutdown_rx, remaining, control.operation).await
}

async fn flush_writer<S>(
    writer: &mut S,
    shutdown_rx: &mut watch::Receiver<bool>,
    timeout: Duration,
    operation: &'static str,
) -> ZaiResult<bool>
where
    S: Sink<Message, Error = WebSocketError> + Unpin,
{
    let flush = tokio::time::timeout(timeout, writer.flush());
    tokio::pin!(flush);
    loop {
        tokio::select! {
            biased;
            changed = shutdown_rx.changed() => {
                if changed.is_err() || *shutdown_rx.borrow() {
                    return Ok(false);
                }
            },
            result = &mut flush => {
                return result
                    .map_err(|_| RealtimeErrorKind::Timeout { operation })?
                    .map(|()| true)
                    .map_err(|source| {
                        warn!("WebSocket flush failed");
                        RealtimeErrorKind::WebSocket { source }.into()
                    });
            },
        }
    }
}

async fn write_frame<S>(
    writer: &mut S,
    message: Message,
    shutdown_rx: &mut watch::Receiver<bool>,
    timeout: Duration,
    operation: &'static str,
) -> ZaiResult<bool>
where
    S: Sink<Message, Error = WebSocketError> + Unpin,
{
    let send = tokio::time::timeout(timeout, writer.send(message));
    tokio::pin!(send);
    loop {
        tokio::select! {
            biased;
            changed = shutdown_rx.changed() => {
                if changed.is_err() || *shutdown_rx.borrow() {
                    return Ok(false);
                }
            },
            result = &mut send => {
                return result
                    .map_err(|_| RealtimeErrorKind::Timeout { operation })?
                    .map(|()| true)
                    .map_err(|source| {
                        warn!("WebSocket send failed");
                        RealtimeErrorKind::WebSocket { source }.into()
                    });
            },
        }
    }
}

#[async_trait]
impl RealtimeTransport for TungsteniteTransport {
    #[tracing::instrument(name = "realtime.send", skip(self, msg))]
    async fn send(&mut self, msg: String) -> ZaiResult<()> {
        self.send_confirmed_frame(msg).await
    }

    #[tracing::instrument(name = "realtime.recv", skip(self))]
    async fn recv(&mut self) -> ZaiResult<Option<WsMessage>> {
        loop {
            let incoming = tokio::select! {
                biased;
                changed = self.writer_status.changed() => {
                    if changed.is_err() {
                        return Err(RealtimeErrorKind::Closed.into());
                    }
                    let status = { self.writer_status.borrow_and_update().clone() };
                    match status {
                        Some(Ok(())) => return Ok(None),
                        Some(Err(error)) => return Err(error),
                        None => continue,
                    }
                },
                incoming = self.reader.next() => incoming,
            };
            match incoming {
                None => {
                    debug!("WebSocket peer closed connection");
                    let _ = self.shutdown_tx.send(true);
                    return Ok(None);
                },
                Some(Err(e)) => {
                    warn!("WebSocket receive failed");
                    let _ = self.shutdown_tx.send(true);
                    return Err(RealtimeErrorKind::WebSocket { source: e }.into());
                },
                Some(Ok(message)) => match message {
                    Message::Text(text) => return Ok(Some(WsMessage::Text(text.to_string()))),
                    Message::Binary(bytes) => return Ok(Some(WsMessage::Binary(bytes))),
                    Message::Ping(_) => {
                        // Coalesce a burst to its latest Ping, as RFC 6455
                        // permits. Tungstenite has already queued the matching
                        // automatic Pong; the dedicated writer path flushes
                        // that shared state with an absolute deadline instead
                        // of adding a second Pong or waiting behind application
                        // data.
                        self.control_tx.send_replace(Some(QueuedControlFlush {
                            operation: "WebSocket pong",
                            deadline: Instant::now() + self.writer_policy.pong_timeout,
                        }));
                        // `reader.next()` may remain immediately ready for a
                        // buffered Ping burst. Yield so the independent writer
                        // can service the deadline-controlled flush before the
                        // next read opportunistically flushes it itself.
                        tokio::task::yield_now().await;
                        continue;
                    },
                    Message::Pong(_) | Message::Frame(_) => continue,
                    Message::Close(_) => {
                        debug!("WebSocket peer closed connection");
                        let _ = self.shutdown_tx.send(true);
                        return Ok(None);
                    },
                },
            }
        }
    }

    #[tracing::instrument(name = "realtime.close", skip(self))]
    async fn close(&mut self) -> ZaiResult<()> {
        if let Some(result) = &self.writer_close_result {
            return result.clone();
        }
        let _ = self.shutdown_tx.send(true);
        if self.writer_join.is_none() {
            return match self.writer_status.borrow().clone() {
                Some(result) => result,
                None => Ok(()),
            };
        }
        let result = join_writer_task(
            &mut self.writer_join,
            self.writer_policy.writer_join_timeout,
        )
        .await;
        self.writer_close_result = Some(result.clone());
        result
    }
}

#[async_trait]
impl RealtimeTransport for BufferedTungsteniteTransport {
    async fn send(&mut self, msg: String) -> ZaiResult<()> {
        self.0.enqueue_text(msg, None)
    }

    async fn send_confirmed(&mut self, msg: String) -> ZaiResult<()> {
        self.0.send_confirmed_frame(msg).await
    }

    async fn recv(&mut self) -> ZaiResult<Option<WsMessage>> {
        self.0.recv().await
    }

    async fn close(&mut self) -> ZaiResult<()> {
        self.0.close().await
    }
}

impl BufferedTungsteniteTransport {
    /// Admit a command whose end-to-end session budgets have already been
    /// reserved. The writer consumes those exact permits instead of attempting
    /// a second byte/count admission that could fail after the caller saw
    /// success.
    pub(crate) fn enqueue_session_text(
        &mut self,
        msg: String,
        budget: SessionFrameBudget,
    ) -> ZaiResult<()> {
        validate_outbound_message(&msg)?;
        let SessionFrameBudget {
            byte_budget,
            slot_budget,
        } = budget;
        let slot_budget = slot_budget.ok_or_else(|| {
            RealtimeErrorKind::Protocol(
                "built-in realtime command is missing its end-to-end slot budget".into(),
            )
        })?;
        self.0
            .enqueue_prebudgeted_text(msg, byte_budget, Some(slot_budget), None)
    }
}

fn validate_outbound_message(msg: &str) -> ZaiResult<()> {
    if msg.len() as u64 > WS_MESSAGE_MAX {
        return Err(RealtimeErrorKind::Protocol(format!(
            "realtime message exceeds {WS_MESSAGE_MAX} bytes"
        ))
        .into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::transport::limits::WS_FRAME_MAX;
    use futures_util::task::AtomicWaker;
    use std::{
        pin::Pin,
        sync::{
            Mutex,
            atomic::{AtomicBool, AtomicUsize, Ordering},
        },
        task::{Context, Poll},
    };

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum SinkEvent {
        Message,
        DataFlush,
        ControlFlush,
    }

    #[derive(Default)]
    struct SinkState {
        messages: Mutex<Vec<Message>>,
        events: Mutex<Vec<SinkEvent>>,
        message_pending: AtomicBool,
        control_flushes: AtomicUsize,
        flush_polls: AtomicUsize,
        block_flush: AtomicBool,
        waker: AtomicWaker,
    }

    impl SinkState {
        fn record_message(&self, item: Message) {
            self.message_pending.store(true, Ordering::Release);
            self.messages.lock().unwrap().push(item);
            self.events.lock().unwrap().push(SinkEvent::Message);
        }

        fn record_flush(&self) -> bool {
            let event = if self.message_pending.swap(false, Ordering::AcqRel) {
                SinkEvent::DataFlush
            } else {
                self.control_flushes.fetch_add(1, Ordering::AcqRel);
                SinkEvent::ControlFlush
            };
            self.events.lock().unwrap().push(event);
            event == SinkEvent::ControlFlush
        }

        fn control_flush_count(&self) -> usize {
            self.control_flushes.load(Ordering::Acquire)
        }
    }

    struct RecordingSink {
        state: Arc<SinkState>,
    }

    impl Sink<Message> for RecordingSink {
        type Error = WebSocketError;

        fn poll_ready(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
        ) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }

        fn start_send(self: Pin<&mut Self>, item: Message) -> Result<(), Self::Error> {
            self.state.record_message(item);
            Ok(())
        }

        fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            self.state.flush_polls.fetch_add(1, Ordering::AcqRel);
            if self.state.block_flush.load(Ordering::Acquire) {
                self.state.waker.register(cx.waker());
                if self.state.block_flush.load(Ordering::Acquire) {
                    return Poll::Pending;
                }
            }
            self.state.record_flush();
            Poll::Ready(Ok(()))
        }

        fn poll_close(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
        ) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }
    }

    #[derive(Clone, Copy)]
    enum FeedbackTrigger {
        ControlFlush,
        Data,
    }

    struct FeedbackSink {
        state: Arc<SinkState>,
        control_tx: watch::Sender<Option<QueuedControlFlush>>,
        trigger: FeedbackTrigger,
        remaining: usize,
    }

    impl FeedbackSink {
        fn replenish_control(&mut self) {
            if self.remaining == 0 {
                return;
            }
            self.remaining -= 1;
            self.control_tx.send_replace(Some(QueuedControlFlush {
                operation: "WebSocket pong",
                deadline: Instant::now() + default_writer_policy().pong_timeout,
            }));
        }
    }

    impl Sink<Message> for FeedbackSink {
        type Error = WebSocketError;

        fn poll_ready(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
        ) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }

        fn start_send(mut self: Pin<&mut Self>, item: Message) -> Result<(), Self::Error> {
            let replenish =
                matches!(self.trigger, FeedbackTrigger::Data) && matches!(&item, Message::Frame(_));
            self.state.record_message(item);
            if replenish {
                self.replenish_control();
            }
            Ok(())
        }

        fn poll_flush(
            mut self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
        ) -> Poll<Result<(), Self::Error>> {
            let control_flush = self.state.record_flush();
            if control_flush && matches!(self.trigger, FeedbackTrigger::ControlFlush) {
                self.replenish_control();
            }
            Poll::Ready(Ok(()))
        }

        fn poll_close(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
        ) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }
    }

    #[derive(Clone, Copy)]
    enum SinkFailure {
        Send,
        Flush,
    }

    struct FailingSink(SinkFailure);

    impl FailingSink {
        fn send() -> Self {
            Self(SinkFailure::Send)
        }

        fn error() -> WebSocketError {
            WebSocketError::Io(std::io::Error::other("test writer failure"))
        }
    }

    impl Sink<Message> for FailingSink {
        type Error = WebSocketError;

        fn poll_ready(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
        ) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }

        fn start_send(self: Pin<&mut Self>, _item: Message) -> Result<(), Self::Error> {
            match self.0 {
                SinkFailure::Send => Err(Self::error()),
                SinkFailure::Flush => Ok(()),
            }
        }

        fn poll_flush(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
        ) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(match self.0 {
                SinkFailure::Send => Ok(()),
                SinkFailure::Flush => Err(Self::error()),
            })
        }

        fn poll_close(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
        ) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }
    }

    fn queued_text(text: String) -> QueuedFrame {
        let permits = u32::try_from(text.len()).unwrap();
        let budget = Arc::new(Semaphore::new(text.len()))
            .try_acquire_many_owned(permits)
            .unwrap();
        QueuedFrame {
            text,
            _byte_budget: budget,
            _slot_budget: None,
            completion: None,
        }
    }

    async fn run_writer_loop_for_test<S>(
        writer: S,
        mut writer_rx: mpsc::Receiver<QueuedFrame>,
        control_rx: watch::Receiver<Option<QueuedControlFlush>>,
        shutdown_rx: watch::Receiver<bool>,
        policy: WriterPolicy,
    ) -> ZaiResult<()>
    where
        S: Sink<Message, Error = WebSocketError> + Unpin,
    {
        writer_loop(writer, &mut writer_rx, control_rx, shutdown_rx, policy).await
    }

    fn default_writer_policy() -> WriterPolicy {
        WriterPolicy::from_transport_config(&RealtimeTransportConfig::default())
    }

    fn queued_pong_flush(deadline: Instant) -> QueuedControlFlush {
        QueuedControlFlush {
            operation: "WebSocket pong",
            deadline,
        }
    }

    fn pending_pong_flush() -> QueuedControlFlush {
        queued_pong_flush(Instant::now() + default_writer_policy().pong_timeout)
    }

    #[test]
    fn writer_policy_uses_the_validated_transport_config() {
        let config = RealtimeTransportConfig::builder()
            .write_timeout(Duration::from_secs(20))
            .pong_timeout(Duration::from_secs(4))
            .close_timeout(Duration::from_secs(3))
            .writer_queue_capacity(3)
            .max_frame_bytes(64 * 1024)
            .try_build()
            .unwrap();
        let policy = WriterPolicy::from_transport_config(&config);

        assert_eq!(policy.write_timeout, Duration::from_secs(20));
        assert_eq!(policy.pong_timeout, Duration::from_secs(4));
        assert_eq!(policy.data_frame_stall_timeout, Duration::from_secs(2));
        assert_eq!(policy.close_timeout, Duration::from_secs(3));
        assert_eq!(policy.confirmed_write_timeout, Duration::from_secs(21));
        assert_eq!(policy.writer_join_timeout, Duration::from_secs(4));
        assert_eq!(policy.max_frame_size, 64 * 1024);
        assert_eq!(config.writer_queue_capacity(), 3);
    }

    #[test]
    fn pong_flushes_have_a_shorter_deadline_than_application_writes() {
        let policy = default_writer_policy();
        assert_eq!(policy.write_timeout, Duration::from_secs(30));
        assert_eq!(policy.pong_timeout, Duration::from_secs(10));
        assert_eq!(policy.close_timeout, Duration::from_secs(5));
        assert_eq!(policy.writer_join_timeout, Duration::from_secs(6));
        assert_eq!(policy.confirmed_write_timeout, Duration::from_secs(31));
        assert_eq!(policy.max_frame_size, WS_FRAME_MAX as usize);
        assert_eq!(policy.data_frame_stall_timeout, Duration::from_secs(5));
        assert!(policy.data_frame_stall_timeout < policy.pong_timeout);
        assert!(policy.pong_timeout < policy.write_timeout);
    }

    #[test]
    fn data_frame_stall_is_independent_and_leaves_pong_headroom() {
        let short_pong = RealtimeTransportConfig::builder()
            .pong_timeout(Duration::from_secs(6))
            .try_build()
            .unwrap();
        let policy = WriterPolicy::from_transport_config(&short_pong);
        assert_eq!(policy.data_frame_stall_timeout, Duration::from_secs(3));
        assert!(policy.data_frame_stall_timeout < policy.pong_timeout);
        assert_eq!(
            Duration::from_secs(2).min(policy.data_frame_stall_timeout),
            Duration::from_secs(2)
        );
    }

    #[test]
    fn websocket_write_buffer_has_a_finite_hard_limit() {
        let config = websocket_config(WS_FRAME_MAX as usize).unwrap();
        assert_eq!(config.max_message_size, Some(WS_MESSAGE_MAX as usize));
        assert_eq!(config.max_frame_size, Some(WS_FRAME_MAX as usize));
        assert_eq!(config.max_write_buffer_size, WS_FRAME_MAX as usize * 2);
        assert!(config.max_write_buffer_size < usize::MAX);
        assert!(config.max_write_buffer_size > config.write_buffer_size);
        assert!(config.max_write_buffer_size >= WS_FRAME_MAX as usize);

        let small_frame = 64 * 1024;
        let small = websocket_config(small_frame).unwrap();
        assert_eq!(small.write_buffer_size, small_frame);
        assert_eq!(small.max_write_buffer_size, small_frame * 2);
        assert!(small.max_write_buffer_size > small.write_buffer_size);
        assert!(websocket_config(usize::MAX).is_err());
    }

    #[test]
    fn outbound_text_is_fragmented_at_the_frame_limit() {
        let text = "界".repeat((WS_FRAME_MAX as usize / '界'.len_utf8()) + 17);
        let expected = text.as_bytes().to_vec();
        let fragments = fragment_text(text, WS_FRAME_MAX as usize).collect::<Vec<Message>>();

        assert!(fragments.len() >= 2);
        let mut rebuilt = Vec::new();
        for (index, message) in fragments.iter().enumerate() {
            let Message::Frame(frame) = message else {
                panic!("outbound fragment was not a raw WebSocket frame");
            };
            assert!(frame.payload().len() <= WS_FRAME_MAX as usize);
            assert_eq!(
                frame.header().opcode,
                OpCode::Data(if index == 0 {
                    Data::Text
                } else {
                    Data::Continue
                })
            );
            assert_eq!(frame.header().is_final, index + 1 == fragments.len());
            rebuilt.extend_from_slice(frame.payload());
        }
        assert_eq!(rebuilt, expected);

        let empty = fragment_text(String::new(), WS_FRAME_MAX as usize).collect::<Vec<_>>();
        assert_eq!(empty.len(), 1);
        let Message::Frame(frame) = &empty[0] else {
            panic!("empty text did not produce a WebSocket frame");
        };
        assert!(frame.payload().is_empty());
        assert_eq!(frame.header().opcode, OpCode::Data(Data::Text));
        assert!(frame.header().is_final);
    }

    #[tokio::test]
    async fn queued_auto_pong_flush_runs_between_text_fragments() {
        let state = Arc::new(SinkState {
            block_flush: AtomicBool::new(true),
            ..SinkState::default()
        });
        let sink = RecordingSink {
            state: Arc::clone(&state),
        };
        let (writer_tx, writer_rx) = mpsc::channel(1);
        let (control_tx, control_rx) = watch::channel(None);
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        writer_tx
            .send(queued_text("x".repeat(WS_FRAME_MAX as usize + 1)))
            .await
            .unwrap();
        let join = tokio::spawn(run_writer_loop_for_test(
            sink,
            writer_rx,
            control_rx,
            shutdown_rx,
            default_writer_policy(),
        ));

        while state.messages.lock().unwrap().is_empty() {
            tokio::task::yield_now().await;
        }
        control_tx.send_replace(Some(pending_pong_flush()));
        state.block_flush.store(false, Ordering::Release);
        state.waker.wake();

        tokio::time::timeout(Duration::from_secs(2), async {
            while state.events.lock().unwrap().len() < 5 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("writer did not resume after the blocked fragment");
        shutdown_tx.send(true).unwrap();
        join.await.unwrap().unwrap();

        let messages = state.messages.lock().unwrap();
        assert_eq!(messages.len(), 2, "control flush emitted a manual Pong");
        let Message::Frame(first) = &messages[0] else {
            panic!("first write was not a text fragment");
        };
        assert_eq!(first.header().opcode, OpCode::Data(Data::Text));
        assert!(!first.header().is_final);
        let Message::Frame(last) = &messages[1] else {
            panic!("last write was not a continuation fragment");
        };
        assert_eq!(last.header().opcode, OpCode::Data(Data::Continue));
        assert!(last.header().is_final);
        assert_eq!(
            state.events.lock().unwrap().as_slice(),
            [
                SinkEvent::Message,
                SinkEvent::DataFlush,
                SinkEvent::ControlFlush,
                SinkEvent::Message,
                SinkEvent::DataFlush,
            ]
        );
    }

    #[tokio::test]
    async fn writer_holds_transferred_session_budgets_until_frame_completion() {
        let state = Arc::new(SinkState {
            block_flush: AtomicBool::new(true),
            ..SinkState::default()
        });
        let sink = RecordingSink {
            state: Arc::clone(&state),
        };
        let byte_pool = Arc::new(Semaphore::new(4));
        let slot_pool = Arc::new(Semaphore::new(1));
        let frame = QueuedFrame {
            text: "data".into(),
            _byte_budget: Arc::clone(&byte_pool).acquire_many_owned(4).await.unwrap(),
            _slot_budget: Some(Arc::clone(&slot_pool).acquire_owned().await.unwrap()),
            completion: None,
        };
        let (writer_tx, writer_rx) = mpsc::channel(1);
        let (_control_tx, control_rx) = watch::channel(None);
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        writer_tx.send(frame).await.unwrap();
        drop(writer_tx);
        let join = tokio::spawn(run_writer_loop_for_test(
            sink,
            writer_rx,
            control_rx,
            shutdown_rx,
            default_writer_policy(),
        ));

        tokio::time::timeout(Duration::from_secs(2), async {
            while state.flush_polls.load(Ordering::Acquire) == 0 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("writer never entered the blocked frame flush");
        assert_eq!(byte_pool.available_permits(), 0);
        assert_eq!(slot_pool.available_permits(), 0);

        state.block_flush.store(false, Ordering::Release);
        state.waker.wake();
        tokio::time::timeout(Duration::from_secs(2), async {
            while byte_pool.available_permits() != 4 || slot_pool.available_permits() != 1 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("writer did not return transferred session budgets");
        shutdown_tx.send(true).unwrap();
        join.await.unwrap().unwrap();
    }

    #[tokio::test(start_paused = true)]
    async fn pong_deadline_includes_queue_wait() {
        let state = Arc::new(SinkState::default());
        let sink = RecordingSink {
            state: Arc::clone(&state),
        };
        let (_writer_tx, writer_rx) = mpsc::channel(1);
        let (control_tx, control_rx) = watch::channel(None);
        let (_shutdown_tx, shutdown_rx) = watch::channel(false);
        let policy = default_writer_policy();
        control_tx.send_replace(Some(queued_pong_flush(
            Instant::now() + policy.pong_timeout,
        )));
        tokio::time::advance(policy.pong_timeout).await;

        let error = run_writer_loop_for_test(sink, writer_rx, control_rx, shutdown_rx, policy)
            .await
            .expect_err("expired Pong was written with a fresh timeout");
        assert!(error.message().contains("WebSocket pong timed out"));
        assert!(state.messages.lock().unwrap().is_empty());
        assert_eq!(state.control_flush_count(), 0);
        assert_eq!(state.flush_polls.load(Ordering::Acquire), 0);
    }

    #[tokio::test(start_paused = true)]
    async fn one_blocked_data_frame_cannot_exceed_pong_sla() {
        let state = Arc::new(SinkState {
            block_flush: AtomicBool::new(true),
            ..SinkState::default()
        });
        let sink = RecordingSink {
            state: Arc::clone(&state),
        };
        let (writer_tx, writer_rx) = mpsc::channel(1);
        let (_control_tx, control_rx) = watch::channel(None);
        let (_shutdown_tx, shutdown_rx) = watch::channel(false);
        writer_tx.send(queued_text("blocked".into())).await.unwrap();
        let join = tokio::spawn(run_writer_loop_for_test(
            sink,
            writer_rx,
            control_rx,
            shutdown_rx,
            default_writer_policy(),
        ));

        while state.messages.lock().unwrap().is_empty() {
            tokio::task::yield_now().await;
        }
        tokio::time::advance(default_writer_policy().data_frame_stall_timeout).await;
        let error = join
            .await
            .unwrap()
            .expect_err("blocked data frame exceeded the Pong SLA");
        assert!(error.message().contains("WebSocket send timed out"));
    }

    #[tokio::test]
    async fn ping_burst_is_coalesced_to_one_automatic_pong_flush() {
        let state = Arc::new(SinkState::default());
        let sink = RecordingSink {
            state: Arc::clone(&state),
        };
        let (_writer_tx, writer_rx) = mpsc::channel(1);
        let (control_tx, control_rx) = watch::channel(None);
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        for _ in 0_u8..=8 {
            control_tx.send_replace(Some(pending_pong_flush()));
        }
        let join = tokio::spawn(run_writer_loop_for_test(
            sink,
            writer_rx,
            control_rx,
            shutdown_rx,
            default_writer_policy(),
        ));

        tokio::time::timeout(Duration::from_secs(2), async {
            while state.control_flush_count() == 0 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("coalesced automatic Pong was not flushed");
        shutdown_tx.send(true).unwrap();
        join.await.unwrap().unwrap();

        assert!(state.messages.lock().unwrap().is_empty());
        assert_eq!(state.control_flush_count(), 1);
    }

    #[tokio::test]
    async fn self_replenishing_ping_stream_cannot_starve_queued_data() {
        let state = Arc::new(SinkState::default());
        let (writer_tx, writer_rx) = mpsc::channel(2);
        let (control_tx, control_rx) = watch::channel(None);
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        control_tx.send_replace(Some(pending_pong_flush()));
        let sink = FeedbackSink {
            state: Arc::clone(&state),
            control_tx,
            trigger: FeedbackTrigger::ControlFlush,
            remaining: 32,
        };
        let join = tokio::spawn(run_writer_loop_for_test(
            sink,
            writer_rx,
            control_rx,
            shutdown_rx,
            default_writer_policy(),
        ));

        tokio::time::timeout(Duration::from_secs(2), async {
            while state.control_flush_count() == 0 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("feedback Ping stream never started");
        // The data lane becomes ready only after the control feedback loop is
        // running. The writer's control-turn yield must let this producer run.
        writer_tx.send(queued_text("first".into())).await.unwrap();
        writer_tx.send(queued_text("second".into())).await.unwrap();

        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                let data_count = state
                    .messages
                    .lock()
                    .unwrap()
                    .iter()
                    .filter(|message| matches!(message, Message::Frame(_)))
                    .count();
                if data_count == 2 {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("self-replenishing Pings starved queued data");
        shutdown_tx.send(true).unwrap();
        join.await.unwrap().unwrap();

        let messages = state.messages.lock().unwrap();
        let data = messages
            .iter()
            .filter_map(|message| {
                let Message::Frame(frame) = message else {
                    return None;
                };
                Some(frame.payload().to_vec())
            })
            .collect::<Vec<_>>();
        assert_eq!(
            data.iter()
                .map(|payload| payload.as_slice())
                .collect::<Vec<_>>(),
            [b"first".as_slice(), b"second".as_slice()]
        );
        // One top-level control turn plus the pre-fragment control check is
        // the maximum before the first ready data frame must progress.
        let scheduled = state
            .events
            .lock()
            .unwrap()
            .iter()
            .copied()
            .filter(|event| *event != SinkEvent::DataFlush)
            .collect::<Vec<_>>();
        let first_data = scheduled
            .iter()
            .position(|event| *event == SinkEvent::Message)
            .expect("writer never scheduled application data");
        assert!(first_data <= 2, "too many control flushes ran before data");
    }

    #[tokio::test]
    async fn data_backlog_cannot_starve_new_control_flushes() {
        const DATA_MESSAGES: usize = 8;

        let state = Arc::new(SinkState::default());
        let (writer_tx, writer_rx) = mpsc::channel(DATA_MESSAGES);
        let (control_tx, control_rx) = watch::channel(None);
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        for index in 0..DATA_MESSAGES {
            writer_tx
                .send(queued_text(format!("data-{index}")))
                .await
                .unwrap();
        }
        let sink = FeedbackSink {
            state: Arc::clone(&state),
            control_tx,
            trigger: FeedbackTrigger::Data,
            remaining: DATA_MESSAGES,
        };
        let join = tokio::spawn(run_writer_loop_for_test(
            sink,
            writer_rx,
            control_rx,
            shutdown_rx,
            default_writer_policy(),
        ));

        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                let complete = {
                    let messages = state.messages.lock().unwrap();
                    let data_count = messages
                        .iter()
                        .filter(|message| matches!(message, Message::Frame(_)))
                        .count();
                    data_count == DATA_MESSAGES && state.control_flush_count() == DATA_MESSAGES
                };
                if complete {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("data backlog starved control flushes");
        shutdown_tx.send(true).unwrap();
        join.await.unwrap().unwrap();

        let messages = state.messages.lock().unwrap();
        assert!(matches!(messages.first(), Some(Message::Frame(_))));
        let scheduled = state
            .events
            .lock()
            .unwrap()
            .iter()
            .copied()
            .filter(|event| *event != SinkEvent::DataFlush)
            .collect::<Vec<_>>();
        assert_eq!(scheduled.first(), Some(&SinkEvent::Message));
        for pair in scheduled.windows(2) {
            assert!(
                !(pair[0] == SinkEvent::Message && pair[1] == SinkEvent::Message),
                "more than one ready data message ran before a control flush"
            );
        }
        let payloads = messages
            .iter()
            .filter_map(|message| match message {
                Message::Frame(frame) => Some(frame.payload().to_vec()),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(
            payloads,
            (0..DATA_MESSAGES)
                .map(|index| format!("data-{index}").into_bytes())
                .collect::<Vec<_>>(),
            "fair scheduling changed application FIFO order"
        );
    }

    #[tokio::test]
    async fn shutdown_remains_highest_when_control_and_data_are_ready() {
        let state = Arc::new(SinkState::default());
        let (writer_tx, writer_rx) = mpsc::channel(1);
        let (control_tx, control_rx) = watch::channel(None);
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        writer_tx.send(queued_text("data".into())).await.unwrap();
        control_tx.send_replace(Some(pending_pong_flush()));
        shutdown_tx.send(true).unwrap();

        run_writer_loop_for_test(
            RecordingSink {
                state: Arc::clone(&state),
            },
            writer_rx,
            control_rx,
            shutdown_rx,
            default_writer_policy(),
        )
        .await
        .unwrap();

        assert!(
            state.events.lock().unwrap().is_empty(),
            "writer touched the sink after shutdown was already ready"
        );
    }

    #[tokio::test]
    async fn shutdown_remains_highest_after_a_control_turn() {
        let state = Arc::new(SinkState::default());
        let (writer_tx, writer_rx) = mpsc::channel(1);
        let (control_tx, control_rx) = watch::channel(None);
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        writer_tx.send(queued_text("data".into())).await.unwrap();
        control_tx.send_replace(Some(pending_pong_flush()));
        let join = tokio::spawn(run_writer_loop_for_test(
            RecordingSink {
                state: Arc::clone(&state),
            },
            writer_rx,
            control_rx,
            shutdown_rx,
            default_writer_policy(),
        ));

        tokio::time::timeout(Duration::from_secs(2), async {
            while state.control_flush_count() == 0 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("initial automatic Pong was not flushed");
        shutdown_tx.send(true).unwrap();
        join.await.unwrap().unwrap();

        assert!(state.messages.lock().unwrap().is_empty());
        assert_eq!(state.control_flush_count(), 1);
    }

    #[tokio::test]
    async fn shutdown_cancels_a_blocked_automatic_pong_flush() {
        let state = Arc::new(SinkState {
            block_flush: AtomicBool::new(true),
            ..SinkState::default()
        });
        let (_writer_tx, writer_rx) = mpsc::channel(1);
        let (control_tx, control_rx) = watch::channel(None);
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        control_tx.send_replace(Some(pending_pong_flush()));
        let join = tokio::spawn(run_writer_loop_for_test(
            RecordingSink {
                state: Arc::clone(&state),
            },
            writer_rx,
            control_rx,
            shutdown_rx,
            default_writer_policy(),
        ));

        tokio::time::timeout(Duration::from_secs(2), async {
            while state.flush_polls.load(Ordering::Acquire) == 0 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("automatic Pong flush never reached the sink");
        shutdown_tx.send(true).unwrap();
        join.await.unwrap().unwrap();

        assert_eq!(state.control_flush_count(), 0);
        assert!(state.messages.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn closing_one_writer_lane_drains_the_other_lane() {
        let data_state = Arc::new(SinkState::default());
        let (writer_tx, writer_rx) = mpsc::channel(1);
        writer_tx.send(queued_text("data".into())).await.unwrap();
        drop(writer_tx);
        let (control_tx, control_rx) = watch::channel(None);
        drop(control_tx);
        let (_shutdown_tx, shutdown_rx) = watch::channel(false);
        run_writer_loop_for_test(
            RecordingSink {
                state: Arc::clone(&data_state),
            },
            writer_rx,
            control_rx,
            shutdown_rx,
            default_writer_policy(),
        )
        .await
        .unwrap();
        assert!(matches!(
            data_state.messages.lock().unwrap().as_slice(),
            [Message::Frame(_)]
        ));

        let control_state = Arc::new(SinkState::default());
        let (writer_tx, writer_rx) = mpsc::channel(1);
        drop(writer_tx);
        let (control_tx, control_rx) = watch::channel(None);
        control_tx.send_replace(Some(pending_pong_flush()));
        let (_shutdown_tx, shutdown_rx) = watch::channel(false);
        let join = tokio::spawn(run_writer_loop_for_test(
            RecordingSink {
                state: Arc::clone(&control_state),
            },
            writer_rx,
            control_rx,
            shutdown_rx,
            default_writer_policy(),
        ));
        tokio::time::timeout(Duration::from_secs(2), async {
            while control_state.control_flush_count() == 0 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("first control flush did not run");
        control_tx.send_replace(Some(pending_pong_flush()));
        drop(control_tx);
        join.await.unwrap().unwrap();

        assert!(control_state.messages.lock().unwrap().is_empty());
        assert_eq!(control_state.control_flush_count(), 2);
    }

    #[tokio::test]
    async fn sink_errors_stop_both_fair_scheduler_lanes() {
        for failure in [SinkFailure::Send, SinkFailure::Flush] {
            let (writer_tx, writer_rx) = mpsc::channel(1);
            let (control_tx, control_rx) = watch::channel(None);
            let (_shutdown_tx, shutdown_rx) = watch::channel(false);
            match failure {
                SinkFailure::Send => writer_tx.send(queued_text("data".into())).await.unwrap(),
                SinkFailure::Flush => {
                    control_tx.send_replace(Some(pending_pong_flush()));
                },
            }

            let error = run_writer_loop_for_test(
                FailingSink(failure),
                writer_rx,
                control_rx,
                shutdown_rx,
                default_writer_policy(),
            )
            .await
            .expect_err("sink failure was hidden by the fair scheduler");
            assert!(matches!(
                error.source_error(),
                crate::ZaiError::RealtimeError(kind)
                    if matches!(kind.as_ref(), RealtimeErrorKind::WebSocket { .. })
            ));
        }
    }

    #[tokio::test]
    async fn writer_status_precedes_receiver_close_observed_by_producers() {
        let (writer_tx, writer_rx) = mpsc::channel(1);
        writer_tx.send(queued_text("data".into())).await.unwrap();
        let (_control_tx, control_rx) = watch::channel(None);
        let (_shutdown_tx, shutdown_rx) = watch::channel(false);
        let (writer_status_tx, writer_status) = watch::channel(None);

        let join = tokio::spawn(writer_task(
            FailingSink::send(),
            writer_rx,
            control_rx,
            shutdown_rx,
            default_writer_policy(),
            writer_status_tx,
        ));

        tokio::time::timeout(Duration::from_secs(2), writer_tx.closed())
            .await
            .expect("writer receiver did not close after the sink failure");

        let published = writer_status
            .borrow()
            .clone()
            .expect("writer receiver closed before its terminal status was published")
            .expect_err("sink failure was published as success");
        assert!(matches!(
            published.source_error(),
            crate::ZaiError::RealtimeError(kind)
                if matches!(kind.as_ref(), RealtimeErrorKind::WebSocket { .. })
        ));
        assert!(matches!(
            writer_tx.try_send(queued_text("late".into())),
            Err(mpsc::error::TrySendError::Closed(_))
        ));

        let joined = join
            .await
            .expect("writer task panicked")
            .expect_err("writer task hid the sink failure");
        assert_eq!(joined.message(), published.message());
    }

    #[tokio::test]
    async fn cancelled_close_wait_retains_the_writer_join_handle() {
        let mut writer_join = Some(tokio::spawn(std::future::pending::<ZaiResult<()>>()));

        let cancelled = tokio::time::timeout(
            Duration::from_millis(1),
            join_writer_task(
                &mut writer_join,
                default_writer_policy().writer_join_timeout,
            ),
        )
        .await;

        assert!(cancelled.is_err());
        let join = writer_join
            .take()
            .expect("cancelling close detached the writer task");
        join.abort();
    }
}
