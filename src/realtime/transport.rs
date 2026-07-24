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

use crate::{
    ZaiResult,
    client::{
        error::RealtimeErrorKind,
        transport::limits::{WS_FRAME_MAX, WS_MESSAGE_MAX},
    },
};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const WRITE_TIMEOUT: Duration = Duration::from_secs(30);
const PONG_TIMEOUT: Duration = Duration::from_secs(10);
const CLOSE_TIMEOUT: Duration = Duration::from_secs(5);
const WRITER_JOIN_TIMEOUT: Duration = Duration::from_secs(6);
const CONFIRMED_WRITE_TIMEOUT: Duration = Duration::from_secs(31);
const WRITER_CHANNEL_CAPACITY: usize = 8;
const WRITER_BUFFER_BYTES_MAX: usize = WS_MESSAGE_MAX as usize;

type WebSocket = WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;
type WebSocketReader = SplitStream<WebSocket>;

struct QueuedFrame {
    text: String,
    _budget: OwnedSemaphorePermit,
    completion: Option<oneshot::Sender<ZaiResult<()>>>,
}

#[derive(Clone)]
struct QueuedControlFrame {
    message: Message,
    operation: &'static str,
    deadline: Instant,
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
    /// The built-in public transport resolves this only after the frame is
    /// written. Session internals use a private bounded adapter so socket
    /// writes cannot block inbound event processing.
    async fn send(&mut self, msg: String) -> ZaiResult<()>;
    /// Receive the next meaningful message, or `None` when the peer closed.
    async fn recv(&mut self) -> ZaiResult<Option<WsMessage>>;
    /// Gracefully close the connection.
    async fn close(&mut self) -> ZaiResult<()>;
}

/// `tokio-tungstenite`-backed WebSocket transport over TLS (rustls).
pub struct TungsteniteTransport {
    reader: WebSocketReader,
    writer_tx: mpsc::Sender<QueuedFrame>,
    control_tx: watch::Sender<Option<QueuedControlFrame>>,
    writer_budget: Arc<Semaphore>,
    shutdown_tx: watch::Sender<bool>,
    writer_status: watch::Receiver<Option<ZaiResult<()>>>,
    writer_join: Option<JoinHandle<ZaiResult<()>>>,
    writer_close_result: Option<ZaiResult<()>>,
}

/// Session-only adapter whose `send` admits frames to the bounded background
/// writer. The public transport retains confirmed-send semantics for callers
/// that use [`RealtimeTransport`] directly.
pub(crate) struct BufferedTungsteniteTransport(TungsteniteTransport);

impl TungsteniteTransport {
    /// Open a WebSocket to `url` with the given `Authorization` header value.
    #[tracing::instrument(name = "realtime.connect", skip_all)]
    pub async fn connect(url: &str, authorization: &str) -> ZaiResult<Self> {
        let mut req = url.into_client_request()?;
        let mut auth_value = HeaderValue::from_str(authorization).map_err(|e| {
            RealtimeErrorKind::Protocol(format!("invalid Authorization value: {e}"))
        })?;
        auth_value.set_sensitive(true);
        req.headers_mut().insert(AUTHORIZATION, auth_value);

        let config = WebSocketConfig::default()
            .max_message_size(Some(WS_MESSAGE_MAX as usize))
            .max_frame_size(Some(WS_FRAME_MAX as usize));
        let (stream, _response) = tokio::time::timeout(
            CONNECT_TIMEOUT,
            connect_async_with_config(req, Some(config), false),
        )
        .await
        .map_err(|_| RealtimeErrorKind::Timeout {
            operation: "WebSocket connect",
        })??;
        let (writer, reader) = stream.split();
        let (writer_tx, writer_rx) = mpsc::channel(WRITER_CHANNEL_CAPACITY);
        let (control_tx, control_rx) = watch::channel(None);
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        let (writer_status_tx, writer_status) = watch::channel(None);
        let writer_join = tokio::spawn(async move {
            let result = writer_loop(writer, writer_rx, control_rx, shutdown_rx).await;
            writer_status_tx.send_replace(Some(result.clone()));
            result
        });
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
        })
    }

    /// Send a frame and wait until the writer has completed it. This is used
    /// for the initial `session.update`; regular session writes use the
    /// transport's bounded asynchronous writer.
    pub(crate) async fn send_confirmed(&mut self, msg: String) -> ZaiResult<()> {
        let (completion, completed) = oneshot::channel();
        self.enqueue_text(msg, Some(completion))?;
        tokio::time::timeout(CONFIRMED_WRITE_TIMEOUT, completed)
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
        if msg.len() as u64 > WS_MESSAGE_MAX {
            return Err(RealtimeErrorKind::Protocol(format!(
                "realtime message exceeds {WS_MESSAGE_MAX} bytes"
            ))
            .into());
        }
        let permits = u32::try_from(msg.len())
            .map_err(|_| RealtimeErrorKind::Protocol("realtime message length overflow".into()))?;
        let budget = Arc::clone(&self.writer_budget)
            .try_acquire_many_owned(permits)
            .map_err(|_| {
                RealtimeErrorKind::Protocol(format!(
                    "realtime outbound writer backlog exceeds {WRITER_BUFFER_BYTES_MAX} bytes"
                ))
            })?;
        let frame = QueuedFrame {
            text: msg,
            _budget: budget,
            completion,
        };
        self.writer_tx.try_send(frame).map_err(|error| match error {
            mpsc::error::TrySendError::Full(_) => RealtimeErrorKind::Protocol(format!(
                "realtime outbound writer queue exceeds {WRITER_CHANNEL_CAPACITY} messages"
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

async fn writer_loop<S>(
    mut writer: S,
    mut writer_rx: mpsc::Receiver<QueuedFrame>,
    mut control_rx: watch::Receiver<Option<QueuedControlFrame>>,
    mut shutdown_rx: watch::Receiver<bool>,
) -> ZaiResult<()>
where
    S: Sink<Message, Error = WebSocketError> + Unpin,
{
    let result = loop {
        tokio::select! {
            biased;
            changed = shutdown_rx.changed() => {
                if changed.is_err() || *shutdown_rx.borrow() {
                    break Ok(());
                }
            },
            changed = control_rx.changed() => {
                if changed.is_err() {
                    break Ok(());
                }
                let control = { control_rx.borrow_and_update().clone() };
                let Some(control) = control else {
                    continue;
                };
                match write_control_frame(
                    &mut writer,
                    control,
                    &mut shutdown_rx,
                )
                .await
                {
                    Ok(true) => {},
                    Ok(false) => break Ok(()),
                    Err(error) => break Err(error),
                }
            },
            frame = writer_rx.recv() => {
                let Some(frame) = frame else {
                    break Ok(());
                };
                let QueuedFrame {
                    text,
                    _budget: budget,
                    completion,
                } = frame;
                let sent = write_text_message(
                    &mut writer,
                    text,
                    &mut control_rx,
                    &mut shutdown_rx,
                )
                .await;
                drop(budget);
                if let Some(completion) = completion {
                    let completed = match &sent {
                        Ok(true) => Ok(()),
                        Ok(false) => Err(RealtimeErrorKind::Closed.into()),
                        Err(error) => Err(error.clone()),
                    };
                    let _ = completion.send(completed);
                }
                match sent {
                    Ok(true) => {},
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
    tokio::time::timeout(CLOSE_TIMEOUT, writer.close())
        .await
        .map_err(|_| RealtimeErrorKind::Timeout {
            operation: "WebSocket close",
        })?
        .map_err(|source| {
            warn!("WebSocket close failed");
            RealtimeErrorKind::WebSocket { source }.into()
        })
}

async fn join_writer_task(writer_join: &mut Option<JoinHandle<ZaiResult<()>>>) -> ZaiResult<()> {
    let Some(join) = writer_join.as_mut() else {
        return Ok(());
    };
    let result = match tokio::time::timeout(WRITER_JOIN_TIMEOUT, &mut *join).await {
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

fn fragment_text(text: String) -> Vec<Message> {
    let payload = Bytes::from(text);
    let frame_max = WS_FRAME_MAX as usize;
    if payload.is_empty() {
        return vec![Message::Frame(Frame::message(
            Bytes::new(),
            OpCode::Data(Data::Text),
            true,
        ))];
    }

    let fragment_count = payload.len().div_ceil(frame_max);
    (0..fragment_count)
        .map(|index| {
            let start = index * frame_max;
            let end = ((index + 1) * frame_max).min(payload.len());
            let opcode = if index == 0 {
                OpCode::Data(Data::Text)
            } else {
                OpCode::Data(Data::Continue)
            };
            Message::Frame(Frame::message(
                payload.slice(start..end),
                opcode,
                index + 1 == fragment_count,
            ))
        })
        .collect()
}

async fn write_text_message<S>(
    writer: &mut S,
    text: String,
    control_rx: &mut watch::Receiver<Option<QueuedControlFrame>>,
    shutdown_rx: &mut watch::Receiver<bool>,
) -> ZaiResult<bool>
where
    S: Sink<Message, Error = WebSocketError> + Unpin,
{
    let message_deadline = Instant::now() + WRITE_TIMEOUT;
    for fragment in fragment_text(text) {
        if control_rx.has_changed().unwrap_or(false) {
            let control = { control_rx.borrow_and_update().clone() };
            if let Some(control) = control
                && !write_control_frame(writer, control, shutdown_rx).await?
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
        if !write_frame(
            writer,
            fragment,
            shutdown_rx,
            remaining.min(PONG_TIMEOUT),
            "WebSocket send",
        )
        .await?
        {
            return Ok(false);
        }
        // Give the reader task a chance to enqueue a Pong before the next
        // continuation frame is selected.
        tokio::task::yield_now().await;
    }
    Ok(true)
}

async fn write_control_frame<S>(
    writer: &mut S,
    control: QueuedControlFrame,
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
    write_frame(
        writer,
        control.message,
        shutdown_rx,
        remaining,
        control.operation,
    )
    .await
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
        self.send_confirmed(msg).await
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
                    Message::Ping(ping) => {
                        // Coalesce a burst to its latest Ping, as RFC 6455
                        // permits. This cannot overflow a queue, and the
                        // dedicated writer path gives the Pong an absolute
                        // deadline instead of waiting behind application data.
                        self.control_tx.send_replace(Some(QueuedControlFrame {
                            message: Message::Pong(ping),
                            operation: "WebSocket pong",
                            deadline: Instant::now() + PONG_TIMEOUT,
                        }));
                        // `reader.next()` may remain immediately ready for a
                        // buffered Ping burst. Yield so the independent writer
                        // can flush the coalesced Pong before more reads.
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
        let result = join_writer_task(&mut self.writer_join).await;
        self.writer_close_result = Some(result.clone());
        result
    }
}

#[async_trait]
impl RealtimeTransport for BufferedTungsteniteTransport {
    async fn send(&mut self, msg: String) -> ZaiResult<()> {
        self.0.enqueue_text(msg, None)
    }

    async fn recv(&mut self) -> ZaiResult<Option<WsMessage>> {
        self.0.recv().await
    }

    async fn close(&mut self) -> ZaiResult<()> {
        self.0.close().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::task::AtomicWaker;
    use std::{
        pin::Pin,
        sync::{
            Mutex,
            atomic::{AtomicBool, Ordering},
        },
        task::{Context, Poll},
    };

    #[derive(Default)]
    struct SinkState {
        messages: Mutex<Vec<Message>>,
        block_flush: AtomicBool,
        waker: AtomicWaker,
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
            self.state.messages.lock().unwrap().push(item);
            Ok(())
        }

        fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            if !self.state.block_flush.load(Ordering::Acquire) {
                return Poll::Ready(Ok(()));
            }
            self.state.waker.register(cx.waker());
            if self.state.block_flush.load(Ordering::Acquire) {
                Poll::Pending
            } else {
                Poll::Ready(Ok(()))
            }
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
            _budget: budget,
            completion: None,
        }
    }

    #[test]
    fn control_frames_have_a_shorter_deadline_than_application_writes() {
        assert_eq!(PONG_TIMEOUT, Duration::from_secs(10));
        assert!(PONG_TIMEOUT < WRITE_TIMEOUT);
    }

    #[test]
    fn outbound_text_is_fragmented_at_the_frame_limit() {
        let text = "界".repeat((WS_FRAME_MAX as usize / '界'.len_utf8()) + 17);
        let expected = text.as_bytes().to_vec();
        let fragments = fragment_text(text);

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
    }

    #[tokio::test]
    async fn queued_pong_runs_between_text_fragments() {
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
        let join = tokio::spawn(writer_loop(sink, writer_rx, control_rx, shutdown_rx));

        while state.messages.lock().unwrap().is_empty() {
            tokio::task::yield_now().await;
        }
        control_tx.send_replace(Some(QueuedControlFrame {
            message: Message::Pong(Bytes::from_static(b"ping")),
            operation: "WebSocket pong",
            deadline: Instant::now() + PONG_TIMEOUT,
        }));
        state.block_flush.store(false, Ordering::Release);
        state.waker.wake();

        tokio::time::timeout(Duration::from_secs(2), async {
            while state.messages.lock().unwrap().len() < 3 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("writer did not resume after the blocked fragment");
        shutdown_tx.send(true).unwrap();
        join.await.unwrap().unwrap();

        let messages = state.messages.lock().unwrap();
        let Message::Frame(first) = &messages[0] else {
            panic!("first write was not a text fragment");
        };
        assert_eq!(first.header().opcode, OpCode::Data(Data::Text));
        assert!(!first.header().is_final);
        assert!(matches!(&messages[1], Message::Pong(payload) if payload.as_ref() == b"ping"));
        let Message::Frame(last) = &messages[2] else {
            panic!("last write was not a continuation fragment");
        };
        assert_eq!(last.header().opcode, OpCode::Data(Data::Continue));
        assert!(last.header().is_final);
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
        control_tx.send_replace(Some(QueuedControlFrame {
            message: Message::Pong(Bytes::new()),
            operation: "WebSocket pong",
            deadline: Instant::now() + PONG_TIMEOUT,
        }));
        tokio::time::advance(PONG_TIMEOUT).await;

        let error = writer_loop(sink, writer_rx, control_rx, shutdown_rx)
            .await
            .expect_err("expired Pong was written with a fresh timeout");
        assert!(error.message().contains("WebSocket pong timed out"));
        assert!(state.messages.lock().unwrap().is_empty());
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
        let join = tokio::spawn(writer_loop(sink, writer_rx, control_rx, shutdown_rx));

        while state.messages.lock().unwrap().is_empty() {
            tokio::task::yield_now().await;
        }
        tokio::time::advance(PONG_TIMEOUT).await;
        let error = join
            .await
            .unwrap()
            .expect_err("blocked data frame exceeded the Pong SLA");
        assert!(error.message().contains("WebSocket send timed out"));
    }

    #[tokio::test]
    async fn ping_burst_is_coalesced_to_the_latest_pong() {
        let state = Arc::new(SinkState::default());
        let sink = RecordingSink {
            state: Arc::clone(&state),
        };
        let (_writer_tx, writer_rx) = mpsc::channel(1);
        let (control_tx, control_rx) = watch::channel(None);
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        for payload in 0_u8..=8 {
            control_tx.send_replace(Some(QueuedControlFrame {
                message: Message::Pong(Bytes::from(vec![payload])),
                operation: "WebSocket pong",
                deadline: Instant::now() + PONG_TIMEOUT,
            }));
        }
        let join = tokio::spawn(writer_loop(sink, writer_rx, control_rx, shutdown_rx));

        tokio::time::timeout(Duration::from_secs(2), async {
            while state.messages.lock().unwrap().is_empty() {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("coalesced Pong was not written");
        shutdown_tx.send(true).unwrap();
        join.await.unwrap().unwrap();

        let messages = state.messages.lock().unwrap();
        assert_eq!(messages.len(), 1);
        assert!(matches!(&messages[0], Message::Pong(payload) if payload.as_ref() == [8]));
    }

    #[tokio::test]
    async fn cancelled_close_wait_retains_the_writer_join_handle() {
        let mut writer_join = Some(tokio::spawn(std::future::pending::<ZaiResult<()>>()));

        let cancelled =
            tokio::time::timeout(Duration::from_millis(1), join_writer_task(&mut writer_join))
                .await;

        assert!(cancelled.is_err());
        let join = writer_join
            .take()
            .expect("cancelling close detached the writer task");
        join.abort();
    }
}
