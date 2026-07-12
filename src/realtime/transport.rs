//! Realtime transport abstraction.
//!
//! [`RealtimeTransport`] is the realtime analogue of the HTTP client trait: it
//! isolates the WebSocket transport behind a small async interface so the
//! session/event-loop logic is testable and transport-agnostic. The default
//! implementation, [`TungsteniteTransport`], uses `tokio-tungstenite` over TLS
//! (rustls).

use async_trait::async_trait;
use bytes::Bytes;
use futures_util::{SinkExt, StreamExt};
use http::{HeaderValue, header::AUTHORIZATION};
use std::time::Duration;
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream, connect_async_with_config,
    tungstenite::{
        client::IntoClientRequest,
        protocol::{Message, WebSocketConfig},
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
    async fn send(&mut self, msg: String) -> ZaiResult<()>;
    /// Receive the next meaningful message, or `None` when the peer closed.
    async fn recv(&mut self) -> ZaiResult<Option<WsMessage>>;
    /// Gracefully close the connection.
    async fn close(&mut self) -> ZaiResult<()>;
}

/// `tokio-tungstenite`-backed WebSocket transport over TLS (rustls).
pub struct TungsteniteTransport {
    inner: WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>,
}

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
        debug!("Realtime WebSocket connected");
        Ok(Self { inner: stream })
    }
}

#[async_trait]
impl RealtimeTransport for TungsteniteTransport {
    #[tracing::instrument(name = "realtime.send", skip(self, msg))]
    async fn send(&mut self, msg: String) -> ZaiResult<()> {
        if msg.len() as u64 > WS_MESSAGE_MAX {
            return Err(RealtimeErrorKind::Protocol(format!(
                "realtime message exceeds {WS_MESSAGE_MAX} bytes"
            ))
            .into());
        }
        tokio::time::timeout(WRITE_TIMEOUT, self.inner.send(Message::text(msg)))
            .await
            .map_err(|_| RealtimeErrorKind::Timeout {
                operation: "WebSocket send",
            })?
            .map_err(|e| {
                // The source can include endpoint or server-provided text; it
                // remains attached to the returned error but is not duplicated
                // into application logs.
                warn!("WebSocket send failed");
                RealtimeErrorKind::WebSocket { source: e }.into()
            })
    }

    #[tracing::instrument(name = "realtime.recv", skip(self))]
    async fn recv(&mut self) -> ZaiResult<Option<WsMessage>> {
        loop {
            match self.inner.next().await {
                None => {
                    debug!("WebSocket peer closed connection");
                    return Ok(None);
                },
                Some(Err(e)) => {
                    warn!("WebSocket receive failed");
                    return Err(RealtimeErrorKind::WebSocket { source: e }.into());
                },
                Some(Ok(message)) => match message {
                    Message::Text(text) => return Ok(Some(WsMessage::Text(text.to_string()))),
                    Message::Binary(bytes) => return Ok(Some(WsMessage::Binary(bytes))),
                    Message::Ping(ping) => {
                        // Echo a pong so the server keeps the session alive.
                        tokio::time::timeout(PONG_TIMEOUT, self.inner.send(Message::Pong(ping)))
                            .await
                            .map_err(|_| RealtimeErrorKind::Timeout {
                                operation: "WebSocket pong",
                            })?
                            .map_err(|e| {
                                warn!("WebSocket pong failed");
                                RealtimeErrorKind::WebSocket { source: e }
                            })?;
                        continue;
                    },
                    Message::Pong(_) | Message::Frame(_) => continue,
                    Message::Close(_) => {
                        debug!("WebSocket peer closed connection");
                        return Ok(None);
                    },
                },
            }
        }
    }

    #[tracing::instrument(name = "realtime.close", skip(self))]
    async fn close(&mut self) -> ZaiResult<()> {
        tokio::time::timeout(CLOSE_TIMEOUT, self.inner.close(None))
            .await
            .map_err(|_| RealtimeErrorKind::Timeout {
                operation: "WebSocket close",
            })?
            .map_err(|e| {
                warn!("WebSocket close failed");
                RealtimeErrorKind::WebSocket { source: e }.into()
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn control_frames_have_a_shorter_deadline_than_application_writes() {
        assert_eq!(PONG_TIMEOUT, Duration::from_secs(10));
        assert!(PONG_TIMEOUT < WRITE_TIMEOUT);
    }
}
