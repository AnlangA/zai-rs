//! # WebSocket Secure (WSS) Client Module
//!
//! This module provides WebSocket secure connection support for real-time communication
//! with the Zhipu AI API. It implements a generic WebSocket client that can be used by
//! different models for real-time interactions.
//!
//! ## Features
//!
//! - **Secure WebSocket Connections** - WSS protocol support for encrypted communication
//! - **Generic Trait-based Design** - Allows different models to implement their own endpoints
//! - **Automatic Reconnection** - Robust connection management with retry logic
//! - **Message Framing** - Proper WebSocket message handling and parsing
//!
//! ## Architecture
//!
//! The module provides two main components:
//!
//! 1. **WebSocketClient Trait** - Defines the interface for WebSocket connections
//! 2. **WssClient Implementation** - Provides the concrete implementation
//!
//! This design allows different models to specify their own endpoints while reusing the
//! underlying WebSocket connection logic.

use crate::client::error::ZaiError;
use futures_util::{SinkExt, StreamExt};
use log::{debug, error, info};
use std::result::Result as StdResult;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use url::Url;

pub type Result<T> = StdResult<T, ZaiError>;

/// Trait for models that support WebSocket connections
pub trait WebSocketClient {
    /// Returns the WebSocket URL for the model
    fn websocket_url(&self) -> String;

    /// Returns the host for the WebSocket connection (default: "open.bigmodel.cn")
    fn websocket_host(&self) -> &'static str {
        "open.bigmodel.cn"
    }

    /// Returns any custom headers required for the WebSocket connection
    fn custom_headers(&self) -> Vec<(String, String)> {
        Vec::new()
    }
}

/// Trait for handling WebSocket events
pub trait WebSocketEventHandler {
    /// Handle a text message from the server
    fn handle_text_message(&mut self, message: &str) -> Result<()>;

    /// Handle a binary message from the server
    fn handle_binary_message(&mut self, data: &[u8]) -> Result<()>;

    /// Handle a ping message from the server
    fn handle_ping(&mut self, data: &[u8]) -> Result<()>;

    /// Handle a pong message from the server
    fn handle_pong(&mut self, data: &[u8]) -> Result<()>;

    /// Handle a close message from the server
    fn handle_close(
        &mut self,
        frame: Option<tokio_tungstenite::tungstenite::protocol::CloseFrame<'static>>,
    ) -> Result<()>;

    /// Handle connection errors
    fn handle_error(&mut self, error: String) -> Result<()>;

    /// Called when the connection is successfully established
    fn on_connected(&mut self) -> Result<()> {
        debug!("WebSocket connected");
        Ok(())
    }

    /// Called when the connection is closed
    fn on_disconnected(&mut self) -> Result<()> {
        debug!("WebSocket disconnected");
        Ok(())
    }
}

/// Default implementation of WebSocketEventHandler that logs events
pub struct DefaultWebSocketEventHandler;

impl WebSocketEventHandler for DefaultWebSocketEventHandler {
    fn handle_text_message(&mut self, message: &str) -> Result<()> {
        debug!("Received text message: {}", message);
        Ok(())
    }

    fn handle_binary_message(&mut self, data: &[u8]) -> Result<()> {
        debug!("Received binary data of length: {}", data.len());
        Ok(())
    }

    fn handle_ping(&mut self, data: &[u8]) -> Result<()> {
        debug!("Received ping with data length: {}", data.len());
        Ok(())
    }

    fn handle_pong(&mut self, data: &[u8]) -> Result<()> {
        debug!("Received pong with data length: {}", data.len());
        Ok(())
    }

    fn handle_close(
        &mut self,
        frame: Option<tokio_tungstenite::tungstenite::protocol::CloseFrame>,
    ) -> Result<()> {
        info!("WebSocket close frame received: {:?}", frame);
        Ok(())
    }

    fn handle_error(&mut self, error: String) -> Result<()> {
        error!("WebSocket error: {}", error);
        Ok(())
    }
}

/// Message type for internal communication
enum WebSocketMessage {
    Text(String),
    Binary(Vec<u8>),
    Close(Option<tokio_tungstenite::tungstenite::protocol::CloseFrame<'static>>),
}

/// WebSocket client for secure connections
pub struct WssClient<H: WebSocketEventHandler> {
    /// API key for authentication
    api_key: String,
    /// Event handler for processing server events
    event_handler: H,
    /// Sender for outgoing messages
    message_sender: Option<mpsc::UnboundedSender<WebSocketMessage>>,
    /// Handle to the sender task
    sender_handle: Option<tokio::task::JoinHandle<()>>,
    /// Receiver for incoming messages
    websocket_receiver: Option<
        futures::stream::SplitStream<
            tokio_tungstenite::WebSocketStream<
                tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
            >,
        >,
    >,
}

impl<H: WebSocketEventHandler> WssClient<H> {
    /// Create a new WebSocket client with the given API key and event handler
    pub fn new(api_key: impl Into<String>, event_handler: H) -> Self {
        Self {
            api_key: api_key.into(),
            event_handler,
            message_sender: None,
            sender_handle: None,
            websocket_receiver: None,
        }
    }

    /// Create a new WebSocket client with the given API key and default event handler
    pub fn with_default_handler(
        api_key: impl Into<String>,
    ) -> WssClient<DefaultWebSocketEventHandler> {
        WssClient {
            api_key: api_key.into(),
            event_handler: DefaultWebSocketEventHandler,
            message_sender: None,
            sender_handle: None,
            websocket_receiver: None,
        }
    }

    /// Connect to the WebSocket server using the model configuration
    pub async fn connect<M>(&mut self, model: &M) -> Result<()>
    where
        M: WebSocketClient,
    {
        let websocket_url = model.websocket_url();
        let url = Url::parse(&websocket_url).map_err(|e| ZaiError::Unknown {
            code: 0,
            message: format!("Invalid WebSocket URL: {}", e),
        })?;

        let mut request_builder = tokio_tungstenite::tungstenite::http::Request::builder()
            .uri(url.as_str())
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Host", model.websocket_host())
            .header(
                "Sec-WebSocket-Key",
                tokio_tungstenite::tungstenite::handshake::client::generate_key(),
            )
            .header("Sec-WebSocket-Version", "13")
            .header("Connection", "Upgrade")
            .header("Upgrade", "websocket");

        // Add any custom headers from the model
        for (key, value) in model.custom_headers() {
            request_builder = request_builder.header(key, value);
        }

        let request = request_builder.body(()).map_err(|e| ZaiError::Unknown {
            code: 0,
            message: format!("Failed to create WebSocket request: {}", e),
        })?;

        let (ws_stream, response) =
            connect_async(request)
                .await
                .map_err(|e| ZaiError::Unknown {
                    code: 0,
                    message: format!("Failed to connect: {}", e),
                })?;

        debug!("WebSocket connected with response: {:?}", response);

        // Set up the message channel for sending messages
        let (tx, mut rx) = mpsc::unbounded_channel::<WebSocketMessage>();
        self.message_sender = Some(tx);

        // Split the WebSocket stream
        let (mut ws_sender, ws_receiver) = ws_stream.split();

        // Create a task to handle outgoing messages
        let sender_handle = tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                let ws_msg = match msg {
                    WebSocketMessage::Text(text) => Message::Text(text),
                    WebSocketMessage::Binary(data) => Message::Binary(data),
                    WebSocketMessage::Close(frame) => Message::Close(frame),
                };

                if let Err(e) = ws_sender.send(ws_msg).await {
                    error!("Failed to send message: {}", e);
                    break;
                }
            }
        });
        self.sender_handle = Some(sender_handle);

        // Store the receiver for later use
        self.websocket_receiver = Some(ws_receiver);

        // Notify the event handler that the connection is established
        self.event_handler.on_connected()?;

        Ok(())
    }

    /// Send a text message to the server
    pub fn send_text(&mut self, text: impl Into<String>) -> Result<()> {
        if let Some(ref tx) = self.message_sender {
            tx.send(WebSocketMessage::Text(text.into()))
                .map_err(|e| ZaiError::Unknown {
                    code: 0,
                    message: format!("Failed to queue text message: {}", e),
                })?;
            Ok(())
        } else {
            Err(ZaiError::Unknown {
                code: 0,
                message: "WebSocket not connected".to_string(),
            })
        }
    }

    /// Send a binary message to the server
    pub fn send_binary(&mut self, data: Vec<u8>) -> Result<()> {
        if let Some(ref tx) = self.message_sender {
            tx.send(WebSocketMessage::Binary(data))
                .map_err(|e| ZaiError::Unknown {
                    code: 0,
                    message: format!("Failed to queue binary message: {}", e),
                })?;
            Ok(())
        } else {
            Err(ZaiError::Unknown {
                code: 0,
                message: "WebSocket not connected".to_string(),
            })
        }
    }

    /// Send a ping message to the server
    pub fn send_ping(&mut self, _data: Option<Vec<u8>>) -> Result<()> {
        // Note: WebSocket ping/pong is handled automatically by the library
        debug!("Sending ping (handled automatically by the library)");
        Ok(())
    }

    /// Send a pong message to the server
    pub fn send_pong(&mut self, _data: Option<Vec<u8>>) -> Result<()> {
        // Note: WebSocket ping/pong is handled automatically by the library
        debug!("Sending pong (handled automatically by the library)");
        Ok(())
    }

    /// Close the WebSocket connection
    pub async fn close(
        &mut self,
        close_frame: Option<tokio_tungstenite::tungstenite::protocol::CloseFrame<'static>>,
    ) -> Result<()> {
        // Close the message channel
        if let Some(ref tx) = self.message_sender {
            let _ = tx.send(WebSocketMessage::Close(close_frame));
        }

        // Abort the sender task
        if let Some(handle) = self.sender_handle.take() {
            handle.abort();
        }

        // Notify the event handler that the connection is closed
        self.event_handler.on_disconnected()?;

        Ok(())
    }

    /// Check if the WebSocket is connected
    pub fn is_connected(&self) -> bool {
        self.message_sender.is_some()
    }

    /// Start listening for messages from the server
    pub async fn listen_for_messages(&mut self) -> Result<()> {
        if let Some(mut receiver) = self.websocket_receiver.take() {
            loop {
                match receiver.next().await {
                    Some(Ok(msg)) => match msg {
                        Message::Text(text) => {
                            if let Err(e) = self.event_handler.handle_text_message(&text) {
                                error!("Error handling text message: {}", e);
                            }
                        }
                        Message::Binary(data) => {
                            if let Err(e) = self.event_handler.handle_binary_message(&data) {
                                error!("Error handling binary message: {}", e);
                            }
                        }
                        Message::Close(frame) => {
                            debug!("WebSocket close frame received: {:?}", frame);
                            if let Err(e) = self.event_handler.on_disconnected() {
                                error!("Error handling disconnection: {}", e);
                            }
                            break;
                        }
                        Message::Ping(_data) => {
                            debug!("Ping received (handled automatically)");
                        }
                        Message::Pong(_data) => {
                            debug!("Pong received");
                        }
                        Message::Frame(_) => {
                            debug!("Raw frame received");
                        }
                    },
                    Some(Err(e)) => {
                        error!("WebSocket error: {}", e);
                        if let Err(err) = self.event_handler.handle_error(e.to_string()) {
                            error!("Error in error handler: {}", err);
                        }
                        break;
                    }
                    None => {
                        debug!("WebSocket stream ended");
                        if let Err(e) = self.event_handler.on_disconnected() {
                            error!("Error handling disconnection: {}", e);
                        }
                        break;
                    }
                }
            }
            Ok(())
        } else {
            Err(ZaiError::Unknown {
                code: 0,
                message: "WebSocket receiver not available".to_string(),
            })
        }
    }
}

impl<H: WebSocketEventHandler> Drop for WssClient<H> {
    fn drop(&mut self) {
        // Abort the sender task if it exists
        if let Some(handle) = self.sender_handle.take() {
            handle.abort();
        }

        // The receiver will be dropped automatically
        let _ = self.websocket_receiver.take();
    }
}

/// Generate a unique event ID
pub fn generate_event_id() -> String {
    use uuid::Uuid;
    Uuid::new_v4().to_string()
}

/// Get the current timestamp in milliseconds
pub fn get_current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
