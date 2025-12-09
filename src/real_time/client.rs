//! # Real-time WebSocket Client
//!
//! This module provides the WebSocket client implementation for the GLM-Realtime API.
//! It handles establishing connections, sending events, and processing responses.

use crate::client::error::ZaiError;
use crate::client::wss::{WssClient, generate_event_id, get_current_timestamp};
use std::result::Result as StdResult;
pub type Result<T> = StdResult<T, ZaiError>;
use crate::real_time::RealtimeModel;
use crate::real_time::types::*;
use base64::Engine;

/// WebSocket client for real-time communication with GLM models
pub struct RealtimeClient {
    /// API key for authentication
    pub api_key: String,
    /// WebSocket client
    websocket_client: Option<WssClient<EventHandlerAdapter>>,
    /// Event handler for processing server events
    event_handler: Box<dyn EventHandler + Send + Sync>,
}

impl RealtimeClient {
    /// Create a new real-time client with the given API key
    pub fn new(api_key: impl Into<String>) -> Self {
        let event_handler = Box::new(DefaultEventHandler);
        Self {
            api_key: api_key.into(),
            websocket_client: None,
            event_handler,
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
        // Create an adapter for the event handler
        // We can't clone the event_handler, so we need to use a different approach
        let event_handler =
            std::mem::replace(&mut self.event_handler, Box::new(DefaultEventHandler));
        let adapter = EventHandlerAdapter::new(event_handler);
        let mut ws_client = WssClient::new(&self.api_key, adapter);

        // Connect using the new WebSocket client
        ws_client.connect(&model).await?;
        self.websocket_client = Some(ws_client);

        // Update the session configuration
        let mut update_config = config;
        update_config.model = Some(model.into());

        self.send_event(ClientEvent::SessionUpdate(SessionUpdateEvent {
            base: BaseClientEvent {
                event_id: Some(generate_event_id()),
                client_timestamp: Some(get_current_timestamp()),
                event_type: "session.update".to_string(),
            },
            session: update_config,
        }))?;

        Ok(())
    }

    /// Send an event to the server
    pub fn send_event(&mut self, event: ClientEvent) -> Result<()> {
        let json = serde_json::to_string(&event)?;

        if let Some(ws) = &mut self.websocket_client {
            ws.send_text(json)?;
        } else {
            return Err(ZaiError::Unknown {
                code: 0,
                message: "WebSocket not connected".to_string(),
            });
        }

        Ok(())
    }

    /// Send audio data to the server
    pub fn send_audio(&mut self, audio_data: &[u8]) -> Result<()> {
        let audio_base64 = base64::engine::general_purpose::STANDARD.encode(audio_data);
        self.send_event(ClientEvent::InputAudioBufferAppend(
            InputAudioBufferAppendEvent {
                base: BaseClientEvent {
                    event_id: Some(generate_event_id()),
                    client_timestamp: Some(get_current_timestamp()),
                    event_type: "input_audio_buffer.append".to_string(),
                },
                audio: audio_base64,
            },
        ))
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
        self.send_event(ClientEvent::InputAudioBufferCommit(
            InputAudioBufferCommitEvent {
                base: BaseClientEvent {
                    event_id: Some(generate_event_id()),
                    client_timestamp: Some(get_current_timestamp()),
                    event_type: "input_audio_buffer.commit".to_string(),
                },
            },
        ))
    }

    /// Clear the audio buffer
    pub fn clear_audio_buffer(&mut self) -> Result<()> {
        self.send_event(ClientEvent::InputAudioBufferClear(
            InputAudioBufferClearEvent {
                base: BaseClientEvent {
                    event_id: Some(generate_event_id()),
                    client_timestamp: Some(get_current_timestamp()),
                    event_type: "input_audio_buffer.clear".to_string(),
                },
            },
        ))
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
        self.send_event(ClientEvent::ConversationItemCreate(
            ConversationItemCreateEvent {
                base: BaseClientEvent {
                    event_id: Some(generate_event_id()),
                    client_timestamp: Some(get_current_timestamp()),
                    event_type: "conversation.item.create".to_string(),
                },
                item,
            },
        ))
    }

    /// Delete a conversation item
    pub fn delete_conversation_item(&mut self, item_id: &str) -> Result<()> {
        self.send_event(ClientEvent::ConversationItemDelete(
            ConversationItemDeleteEvent {
                base: BaseClientEvent {
                    event_id: Some(generate_event_id()),
                    client_timestamp: Some(get_current_timestamp()),
                    event_type: "conversation.item.delete".to_string(),
                },
                item_id: item_id.to_string(),
            },
        ))
    }

    /// Retrieve a conversation item
    pub fn retrieve_conversation_item(&mut self, item_id: &str) -> Result<()> {
        self.send_event(ClientEvent::ConversationItemRetrieve(
            ConversationItemRetrieveEvent {
                base: BaseClientEvent {
                    event_id: Some(generate_event_id()),
                    client_timestamp: Some(get_current_timestamp()),
                    event_type: "conversation.item.retrieve".to_string(),
                },
                item_id: item_id.to_string(),
            },
        ))
    }

    /// Start listening for server events
    pub async fn listen_for_events(&mut self) -> Result<()> {
        if let Some(ws) = &mut self.websocket_client {
            ws.listen_for_messages().await?;
            Ok(())
        } else {
            Err(ZaiError::Unknown {
                code: 0,
                message: "WebSocket not connected".to_string(),
            })
        }
    }
}

impl Clone for RealtimeClient {
    // Note: Implementing Clone for RealtimeClient is challenging because
    // Box<dyn EventHandler> doesn't implement Clone. This would require
    // a more complex approach in a production environment.
    // For now, we'll implement a simplified version that creates a new DefaultEventHandler.
    fn clone(&self) -> Self {
        Self {
            api_key: self.api_key.clone(),
            websocket_client: None,
            event_handler: Box::new(DefaultEventHandler),
        }
    }
}
