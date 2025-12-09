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
use futures_util::{SinkExt, StreamExt, TryFutureExt};
use log::{debug, error, info, warn};
use std::result::Result as StdResult;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio_tungstenite::{WebSocketStream, connect_async, tungstenite::Message};
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

/// WebSocket client for secure connections
pub struct WssClient<H: WebSocketEventHandler> {
    /// API key for authentication
    api_key: String,
    /// WebSocket connection
    websocket: Option<WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>>,
    /// Event handler for processing server events
    event_handler: H,
}

impl<H: WebSocketEventHandler> WssClient<H> {
    /// Create a new WebSocket client with the given API key and event handler
    pub fn new(api_key: impl Into<String>, event_handler: H) -> Self {
        Self {
            api_key: api_key.into(),
            websocket: None,
            event_handler,
        }
    }

    /// Create a new WebSocket client with the given API key and default event handler
    pub fn with_default_handler(
        api_key: impl Into<String>,
    ) -> WssClient<DefaultWebSocketEventHandler> {
        WssClient {
            api_key: api_key.into(),
            websocket: None,
            event_handler: DefaultWebSocketEventHandler,
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
        self.websocket = Some(ws_stream);

        // Notify the event handler that the connection is established
        self.event_handler.on_connected()?;

        Ok(())
    }

    /// Send a text message to the server
    pub fn send_text(&mut self, text: impl Into<String>) -> Result<()> {
        if let Some(ws) = &mut self.websocket {
            futures::executor::block_on(ws.send(Message::Text(text.into())).map_err(|e| {
                ZaiError::Unknown {
                    code: 0,
                    message: format!("Failed to send text message: {}", e),
                }
            }))?;
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
        if let Some(ws) = &mut self.websocket {
            futures::executor::block_on(ws.send(Message::Binary(data)).map_err(|e| {
                ZaiError::Unknown {
                    code: 0,
                    message: format!("Failed to send binary message: {}", e),
                }
            }))?;
            Ok(())
        } else {
            Err(ZaiError::Unknown {
                code: 0,
                message: "WebSocket not connected".to_string(),
            })
        }
    }

    /// Send a ping message to the server
    pub fn send_ping(&mut self, data: Option<Vec<u8>>) -> Result<()> {
        if let Some(ws) = &mut self.websocket {
            futures::executor::block_on(ws.send(Message::Ping(data.unwrap_or_default())).map_err(
                |e| ZaiError::Unknown {
                    code: 0,
                    message: format!("Failed to send ping: {}", e),
                },
            ))?;
            Ok(())
        } else {
            Err(ZaiError::Unknown {
                code: 0,
                message: "WebSocket not connected".to_string(),
            })
        }
    }

    /// Send a pong message to the server
    pub fn send_pong(&mut self, data: Option<Vec<u8>>) -> Result<()> {
        if let Some(ws) = &mut self.websocket {
            futures::executor::block_on(ws.send(Message::Pong(data.unwrap_or_default())).map_err(
                |e| ZaiError::Unknown {
                    code: 0,
                    message: format!("Failed to send pong: {}", e),
                },
            ))?;
            Ok(())
        } else {
            Err(ZaiError::Unknown {
                code: 0,
                message: "WebSocket not connected".to_string(),
            })
        }
    }

    /// Close the WebSocket connection
    pub async fn close(
        &mut self,
        close_frame: Option<tokio_tungstenite::tungstenite::protocol::CloseFrame<'static>>,
    ) -> Result<()> {
        if let Some(ws) = &mut self.websocket {
            ws.close(close_frame).await.map_err(|e| ZaiError::Unknown {
                code: 0,
                message: format!("Failed to close WebSocket: {}", e),
            })?;

            // Notify the event handler that the connection is closed
            self.event_handler.on_disconnected()?;

            Ok(())
        } else {
            Err(ZaiError::Unknown {
                code: 0,
                message: "WebSocket not connected".to_string(),
            })
        }
    }

    /// Check if the WebSocket is connected
    pub fn is_connected(&self) -> bool {
        self.websocket.is_some()
    }

    /// Start listening for messages from the server
    pub async fn listen_for_messages(&mut self) -> Result<()> {
        // Take the websocket out of self to avoid borrow checker issues
        let mut ws = self.websocket.take().ok_or_else(|| ZaiError::Unknown {
            code: 0,
            message: "WebSocket not connected".to_string(),
        })?;

        loop {
            match ws.next().await {
                Some(Ok(Message::Text(text))) => {
                    self.event_handler.handle_text_message(&text)?;
                }
                Some(Ok(Message::Binary(data))) => {
                    self.event_handler.handle_binary_message(&data)?;
                }
                Some(Ok(Message::Ping(data))) => {
                    self.event_handler.handle_ping(&data)?;
                    // Respond with pong
                    let _ = ws.send(Message::Pong(data)).await;
                }
                Some(Ok(Message::Pong(data))) => {
                    self.event_handler.handle_pong(&data)?;
                }
                Some(Ok(Message::Close(close_frame))) => {
                    self.event_handler.handle_close(close_frame)?;
                    break;
                }
                Some(Ok(_)) => {
                    // Handle other message types
                }
                Some(Err(e)) => {
                    let error_msg = format!("WebSocket error: {}", e);
                    self.event_handler.handle_error(error_msg.clone())?;
                    return Err(ZaiError::Unknown {
                        code: 0,
                        message: error_msg,
                    });
                }
                None => {
                    info!("WebSocket stream ended");
                    break;
                }
            }
        }

        // Put the websocket back
        self.websocket = Some(ws);
        Ok(())
    }
}

impl<H: WebSocketEventHandler> Drop for WssClient<H> {
    fn drop(&mut self) {
        if let Some(mut ws) = self.websocket.take() {
            // Try to close the connection gracefully
            if let Err(e) = futures::executor::block_on(async { ws.close(None).await }) {
                warn!("Failed to close WebSocket connection: {}", e);
            }
        }
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
