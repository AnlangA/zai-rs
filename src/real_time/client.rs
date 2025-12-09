//! # Real-time WebSocket Client
//!
//! This module provides the WebSocket client implementation for the GLM-Realtime API.
//! It handles establishing connections, sending events, and processing responses.

use crate::client::error::{Error, Result};
use crate::real_time::types::*;
use futures_util::{SinkExt, StreamExt};
use log::{debug, error, info, warn};
use serde_json::{json, Value};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio_tungstenite::{connect_async, tungstenite::Message, WebSocketStream};
use url::Url;

/// WebSocket client for real-time communication with GLM models
pub struct RealtimeClient {
    /// API key for authentication
    api_key: String,
    /// WebSocket connection
    websocket: Option<WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>>,
    /// Event handler for processing server events
    event_handler: Box<dyn EventHandler + Send + Sync>,
}

impl RealtimeClient {
    /// Create a new real-time client with the given API key
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            websocket: None,
            event_handler: Box::new(DefaultEventHandler),
        }
    }

    /// Set a custom event handler
    pub fn with_event_handler<H>(mut self, handler: H) -> Self
    where
        H: EventHandler + Send + Sync + 'static,
    {
        self.event_handler = Box::new(handler);
        self
    }

    /// Connect to the real-time API with the given session configuration
    pub async fn connect<M>(&mut self, model: M, config: SessionConfig) -> Result<()>
    where
        M: RealtimeModel,
    {
        let websocket_url = model.websocket_url();
        let url = Url::parse(&websocket_url)
            .map_err(|e| Error::IoError(format!("Invalid WebSocket URL: {}", e)))?;

        let request = tokio_tungstenite::tungstenite::http::Request::builder()
            .uri(url.as_str())
            .header("Authorization", format!("Bearer {}", self.api_key))
            .body(())
            .map_err(|e| Error::IoError(format!("Failed to create WebSocket request: {}", e)))?;

        let (ws_stream, response) = connect_async(request)
            .await
            .map_err(|e| Error::ConnectionError(format!("Failed to connect: {}", e)))?;

        debug!("WebSocket connected with response: {:?}", response);
        self.websocket = Some(ws_stream);

        // Update the session configuration
        let mut update_config = config;
        update_config.model = Some(model.model().to_string());

        self.send_event(ClientEvent::SessionUpdate(SessionUpdateEvent {
            base: BaseClientEvent {
                event_id: Some(generate_event_id()),
                client_timestamp: Some(get_current_timestamp()),
                event_type: "session.update".to_string(),
            },
            session: update_config,
        ))?;

        Ok(())
    }

    /// Send an event to the server
    pub fn send_event(&mut self, event: ClientEvent) -> Result<()> {
        let json = serde_json::to_string(&event)
            .map_err(|e| Error::SerializationError(e.to_string()))?;

        if let Some(ws) = &mut self.websocket {
            ws.send(Message::Text(json))
                .map_err(|e| Error::ConnectionError(format!("Failed to send event: {}", e)))?;
        } else {
            return Err(Error::ConnectionError("WebSocket not connected".to_string()));
        }

        Ok(())
    }

    /// Send audio data to the server
    pub fn send_audio(&mut self, audio_data: &[u8]) -> Result<()> {
        let audio_base64 = base64::encode(audio_data);
        self.send_event(ClientEvent::InputAudioBufferAppend(InputAudioBufferAppendEvent {
            base: BaseClientEvent {
                event_id: Some(generate_event_id()),
                client_timestamp: Some(get_current_timestamp()),
                event_type: "input_audio_buffer.append".to_string(),
            },
            audio: audio_base64,
        }))
    }

    /// Send a video frame to the server (base64 encoded jpg)
    pub fn send_video_frame(&mut self, video_frame: &str) -> Result<()> {
        self.send_event(ClientEvent::InputAudioBufferAppendVideoFrame(
            InputAudioBufferAppendVideoFrameEvent {
                base: BaseClientEvent {
                    event_id: Some(generate_event_id()),
                    client_timestamp: Some(get_current_timestamp()),
                    event_type: "input_audio_buffer.append_video_frame".to_string(),
                },
                video_frame: video_frame.to_string(),
            },
        ))
    }

    /// Commit the audio buffer to create a response
    pub fn commit_audio_buffer(&mut self) -> Result<()> {
        self.send_event(ClientEvent::InputAudioBufferCommit(InputAudioBufferCommitEvent {
            base: BaseClientEvent {
                event_id: Some(generate_event_id()),
                client_timestamp: Some(get_current_timestamp()),
                event_type: "input_audio_buffer.commit".to_string(),
            },
        }))
    }

    /// Clear the audio buffer
    pub fn clear_audio_buffer(&mut self) -> Result<()> {
        self.send_event(ClientEvent::InputAudioBufferClear(InputAudioBufferClearEvent {
            base: BaseClientEvent {
                event_id: Some(generate_event_id()),
                client_timestamp: Some(get_current_timestamp()),
                event_type: "input_audio_buffer.clear".to_string(),
            },
        }))
    }

    /// Create a response from the model
    pub fn create_response(&mut self) -> Result<()> {
        self.send_event(ClientEvent::ResponseCreate(ResponseCreateEvent {
            base: BaseClientEvent {
                event_id: Some(generate_event_id()),
                client_timestamp: Some(get_current_timestamp()),
                event_type: "response.create".to_string(),
            },
        }))
    }

    /// Cancel the current response
    pub fn cancel_response(&mut self) -> Result<()> {
        self.send_event(ClientEvent::ResponseCancel(ResponseCancelEvent {
            base: BaseClientEvent {
                event_id: Some(generate_event_id()),
                client_timestamp: Some(get_current_timestamp()),
                event_type: "response.cancel".to_string(),
            },
        }))
    }

    /// Create a conversation item
    pub fn create_conversation_item(&mut self, item: RealtimeConversationItem) -> Result<()> {
        self.send_event(ClientEvent::ConversationItemCreate(ConversationItemCreateEvent {
            base: BaseClientEvent {
                event_id: Some(generate_event_id()),
                client_timestamp: Some(get_current_timestamp()),
                event_type: "conversation.item.create".to_string(),
            },
            item,
        }))
    }

    /// Delete a conversation item
    pub fn delete_conversation_item(&mut self, item_id: &str) -> Result<()> {
        self.send_event(ClientEvent::ConversationItemDelete(ConversationItemDeleteEvent {
            base: BaseClientEvent {
                event_id: Some(generate_event_id()),
                client_timestamp: Some(get_current_timestamp()),
                event_type: "conversation.item.delete".to_string(),
            },
            item_id: item_id.to_string(),
        }))
    }

    /// Retrieve a conversation item
    pub fn retrieve_conversation_item(&mut self, item_id: &str) -> Result<()> {
        self.send_event(ClientEvent::ConversationItemRetrieve(ConversationItemRetrieveEvent {
            base: BaseClientEvent {
                event_id: Some(generate_event_id()),
                client_timestamp: Some(get_current_timestamp()),
                event_type: "conversation.item.retrieve".to_string(),
            },
            item_id: item_id.to_string(),
        }))
    }

    /// Start listening for server events
    pub async fn listen_for_events(&mut self) -> Result<()> {
        if let Some(ws) = &mut self.websocket {
            loop {
                match ws.next().await {
                    Some(Ok(Message::Text(text))) => {
                        debug!("Received message: {}", text);
                        self.handle_server_message(text).await?;
                    }
                    Some(Ok(Message::Binary(data))) => {
                        debug!("Received binary data of length: {}", data.len());
                        // Handle binary messages if needed
                    }
                    Some(Ok(Message::Ping(data))) => {
                        debug!("Received ping");
                        // Respond with pong
                        ws.send(Message::Pong(data))
                            .await
                            .map_err(|e| Error::ConnectionError(format!("Failed to send pong: {}", e)))?;
                    }
                    Some(Ok(Message::Pong(_))) => {
                        debug!("Received pong");
                    }
                    Some(Ok(Message::Close(close_frame))) => {
                        info!("WebSocket closed: {:?}", close_frame);
                        break;
                    }
                    Some(Err(e)) => {
                        error!("WebSocket error: {}", e);
                        return Err(Error::ConnectionError(format!("WebSocket error: {}", e)));
                    }
                    None => {
                        info!("WebSocket stream ended");
                        break;
                    }
                    _ => {}
                }
            }
        } else {
            return Err(Error::ConnectionError("WebSocket not connected".to_string()));
        }

        Ok(())
    }

    /// Handle a server message
    async fn handle_server_message(&mut self, message: String) -> Result<()> {
        // Try to parse as a known server event
        match serde_json::from_str::<ServerEvent>(&message) {
            Ok(event) => {
                match event {
                    ServerEvent::Error(event) => {
                        self.event_handler.on_error(event);
                    }
                    ServerEvent::SessionCreated(event) => {
                        self.event_handler.on_session_created(event);
                    }
                    ServerEvent::SessionUpdated(event) => {
                        self.event_handler.on_session_updated(event);
                    }
                    ServerEvent::ResponseTextDelta(event) => {
                        self.event_handler.on_response_text_delta(event);
                    }
                    ServerEvent::ResponseTextDone(event) => {
                        self.event_handler.on_response_text_done(event);
                    }
                    ServerEvent::ResponseAudioDelta(event) => {
                        self.event_handler.on_response_audio_delta(event);
                    }
                    ServerEvent::ResponseAudioDone(event) => {
                        self.event_handler.on_response_audio_done(event);
                    }
                    ServerEvent::ResponseDone(event) => {
                        self.event_handler.on_response_done(event);
                    }
                    ServerEvent::Heartbeat(event) => {
                        self.event_handler.on_heartbeat(event);
                    }
                    _ => {
                        // Handle other events
                        self.handle_other_server_events(event).await?;
                    }
                }
            }
            Err(e) => {
                // If parsing fails, try to parse as a generic JSON value
                match serde_json::from_str::<Value>(&message) {
                    Ok(value) => {
                        warn!("Failed to parse as known event: {}. Treating as unknown event.", e);
                        self.event_handler.on_unknown_event(value);
                    }
                    Err(e) => {
                        error!("Failed to parse message as JSON: {}", e);
                    }
                }
            }
        }

        Ok(())
    }

    /// Handle server events that don't have dedicated methods in the EventHandler trait
    async fn handle_other_server_events(&mut self, event: ServerEvent) -> Result<()> {
        match event {
            ServerEvent::TranscriptionSessionUpdated(event) => {
                debug!("Transcription session updated: {:?}", event);
            }
            ServerEvent::ConversationItemCreated(event) => {
                debug!("Conversation item created: {:?}", event);
            }
            ServerEvent::ConversationItemDeleted(event) => {
                debug!("Conversation item deleted: {:?}", event);
            }
            ServerEvent::ConversationItemRetrieved(event) => {
                debug!("Conversation item retrieved: {:?}", event);
            }
            ServerEvent::InputAudioTranscriptionCompleted(event) => {
                debug!("Input audio transcription completed: {:?}", event);
            }
            ServerEvent::InputAudioTranscriptionFailed(event) => {
                debug!("Input audio transcription failed: {:?}", event);
            }
            ServerEvent::InputAudioBufferCommitted(event) => {
                debug!("Input audio buffer committed: {:?}", event);
            }
            ServerEvent::InputAudioBufferCleared(event) => {
                debug!("Input audio buffer cleared: {:?}", event);
            }
            ServerEvent::InputAudioBufferSpeechStarted(event) => {
                debug!("Input audio buffer speech started: {:?}", event);
            }
            ServerEvent::InputAudioBufferSpeechStopped(event) => {
                debug!("Input audio buffer speech stopped: {:?}", event);
            }
            ServerEvent::ResponseOutputItemAdded(event) => {
                debug!("Response output item added: {:?}", event);
            }
            ServerEvent::ResponseOutputItemDone(event) => {
                debug!("Response output item done: {:?}", event);
            }
            ServerEvent::ResponseContentPartAdded(event) => {
                debug!("Response content part added: {:?}", event);
            }
            ServerEvent::ResponseContentPartDone(event) => {
                debug!("Response content part done: {:?}", event);
            }
            ServerEvent::ResponseFunctionCallArgumentsDone(event) => {
                debug!("Response function call arguments done: {:?}", event);
            }
            ServerEvent::ResponseFunctionCallSimpleBrowser(event) => {
                debug!("Response function call simple browser: {:?}", event);
            }
            ServerEvent::ResponseAudioTranscriptDelta(event) => {
                debug!("Response audio transcript delta: {:?}", event);
            }
            ServerEvent::ResponseAudioTranscriptDone(event) => {
                debug!("Response audio transcript done: {:?}", event);
            }
            ServerEvent::ResponseCreated(event) => {
                debug!("Response created: {:?}", event);
            }
            ServerEvent::ResponseCancelled(event) => {
                debug!("Response cancelled: {:?}", event);
            }
            ServerEvent::RateLimitsUpdated(event) => {
                debug!("Rate limits updated: {:?}", event);
            }
            _ => {
                // This should never happen if the enum is exhaustive
                warn!("Unhandled server event: {:?}", event);
            }
        }

        Ok(())
    }
}

impl Drop for RealtimeClient {
    fn drop(&mut self) {
        if let Some(mut ws) = self.websocket.take() {
            // Try to close the connection gracefully
            if let Err(e) = futures::executor::block_on(async {
                ws.close(None).await
            }) {
                warn!("Failed to close WebSocket connection: {}", e);
            }
        }
    }
}

/// Generate a unique event ID
fn generate_event_id() -> String {
    use uuid::Uuid;
    Uuid::new_v4().to_string()
}

/// Get the current timestamp in milliseconds
fn get_current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
