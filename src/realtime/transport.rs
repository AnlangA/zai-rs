//! Realtime transport abstraction.
//!
//! [`RealtimeTransport`] is the realtime analogue of the HTTP [`HttpClient`]
//! trait: it isolates the WebSocket transport behind a small async interface so
//! the session/event-loop logic is testable and transport-agnostic. The default
//! implementation, [`TungsteniteTransport`], uses `tokio-tungstenite` over TLS
//! (rustls).
//!
//! [`HttpClient`]: crate::client::http::HttpClient

use async_trait::async_trait;
use bytes::Bytes;
use futures::{SinkExt, StreamExt};
use http::{HeaderValue, header::AUTHORIZATION};
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream, connect_async,
    tungstenite::{client::IntoClientRequest, protocol::Message},
};

use crate::{ZaiResult, client::error::RealtimeErrorKind};

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
    pub async fn connect(url: &str, authorization: &str) -> ZaiResult<Self> {
        let mut req = url.into_client_request()?;
        let auth_value = HeaderValue::from_str(authorization).map_err(|e| {
            RealtimeErrorKind::Protocol(format!("invalid Authorization value: {e}"))
        })?;
        req.headers_mut().insert(AUTHORIZATION, auth_value);

        let (stream, _response) = connect_async(req).await?;
        Ok(Self { inner: stream })
    }
}

#[async_trait]
impl RealtimeTransport for TungsteniteTransport {
    async fn send(&mut self, msg: String) -> ZaiResult<()> {
        self.inner
            .send(Message::text(msg))
            .await
            .map_err(|e| RealtimeErrorKind::WebSocket(e.to_string()).into())
    }

    async fn recv(&mut self) -> ZaiResult<Option<WsMessage>> {
        loop {
            match self.inner.next().await {
                None => return Ok(None),
                Some(Err(e)) => {
                    return Err(RealtimeErrorKind::WebSocket(e.to_string()).into());
                },
                Some(Ok(message)) => match message {
                    Message::Text(text) => return Ok(Some(WsMessage::Text(text.to_string()))),
                    Message::Binary(bytes) => return Ok(Some(WsMessage::Binary(bytes))),
                    Message::Ping(ping) => {
                        // Echo a pong so the server keeps the session alive.
                        let _ = self.inner.send(Message::Pong(ping)).await;
                        continue;
                    },
                    Message::Pong(_) | Message::Frame(_) => continue,
                    Message::Close(_) => return Ok(None),
                },
            }
        }
    }

    async fn close(&mut self) -> ZaiResult<()> {
        self.inner
            .close(None)
            .await
            .map_err(|e| RealtimeErrorKind::WebSocket(e.to_string()).into())
    }
}
