//! Client-to-server events for GLM-Realtime API.
//!
//! This module defines all event types that can be sent from the client
//! to the server in a real-time conversation.

use serde::{Deserialize, Serialize};
use validator::Validate;

// Import everything from the real_time module to simplify references
use crate::real_time::session::*;
use crate::real_time::types::*;

/// Represents a client event sent to the server.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientEvent {
    /// Updates the session configuration.
    #[serde(rename = "session.update")]
    SessionUpdate(SessionUpdateEvent),
    /// Updates the transcription session configuration.
    #[serde(rename = "transcription_session.update")]
    TranscriptionSessionUpdate(TranscriptionSessionUpdateEvent),
    /// Appends audio data to the input buffer.
    #[serde(rename = "input_audio_buffer.append")]
    InputAudioBufferAppend(InputAudioBufferAppendEvent),
    /// Appends a video frame to the input buffer.
    #[serde(rename = "input_audio_buffer.append_video_frame")]
    InputAudioBufferAppendVideoFrame(InputAudioBufferAppendVideoFrameEvent),
    /// Commits the audio data in the input buffer.
    #[serde(rename = "input_audio_buffer.commit")]
    InputAudioBufferCommit(InputAudioBufferCommitEvent),
    /// Clears the audio data in the input buffer.
    #[serde(rename = "input_audio_buffer.clear")]
    InputAudioBufferClear(InputAudioBufferClearEvent),
    /// Creates a new conversation item.
    #[serde(rename = "conversation.item.create")]
    ConversationItemCreate(ConversationItemCreateEvent),
    /// Deletes a conversation item.
    #[serde(rename = "conversation.item.delete")]
    ConversationItemDelete(ConversationItemDeleteEvent),
    /// Retrieves a conversation item.
    #[serde(rename = "conversation.item.retrieve")]
    ConversationItemRetrieve(ConversationItemRetrieveEvent),
    /// Creates a new response.
    #[serde(rename = "response.create")]
    ResponseCreate(ResponseCreateEvent),
    /// Cancels the current response.
    #[serde(rename = "response.cancel")]
    ResponseCancel(ResponseCancelEvent),
}

// Implement getter and setter methods for SessionUpdateEvent
impl SessionUpdateEvent {
    /// Get the event_id
    pub fn get_event_id(&self) -> Option<&String> {
        self.event_id.as_ref()
    }

    /// Set the event_id
    pub fn set_event_id(&mut self, event_id: Option<String>) {
        self.event_id = event_id;
    }

    /// Get the client_timestamp
    pub fn get_client_timestamp(&self) -> Option<u64> {
        self.client_timestamp
    }

    /// Set the client_timestamp
    pub fn set_client_timestamp(&mut self, client_timestamp: Option<u64>) {
        self.client_timestamp = client_timestamp;
    }

    /// Get the session
    pub fn get_session(&self) -> &Session {
        &self.session
    }

    /// Set the session
    pub fn set_session(&mut self, session: Session) {
        self.session = session;
    }

    /// Serialize to JSON string
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Deserialize from JSON string
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

/// Event for updating session configuration.
#[derive(Clone, Debug, Serialize, Deserialize, Validate, Default)]
#[serde(default)]
pub struct SessionUpdateEvent {
    /// Unique identifier for the event.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_id: Option<String>,

    /// Client timestamp in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_timestamp: Option<u64>,

    /// Session configuration.
    pub session: Session,
}

// Implement getter and setter methods for TranscriptionSessionUpdateEvent
impl TranscriptionSessionUpdateEvent {
    /// Get the event_id
    pub fn get_event_id(&self) -> Option<&String> {
        self.event_id.as_ref()
    }

    /// Set the event_id
    pub fn set_event_id(&mut self, event_id: Option<String>) {
        self.event_id = event_id;
    }

    /// Get the client_timestamp
    pub fn get_client_timestamp(&self) -> Option<u64> {
        self.client_timestamp
    }

    /// Set the client_timestamp
    pub fn set_client_timestamp(&mut self, client_timestamp: Option<u64>) {
        self.client_timestamp = client_timestamp;
    }

    /// Get the session
    pub fn get_session(&self) -> &TranscriptionSession {
        &self.session
    }

    /// Set the session
    pub fn set_session(&mut self, session: TranscriptionSession) {
        self.session = session;
    }

    /// Serialize to JSON string
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Deserialize from JSON string
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

// Implement getter and setter methods for TranscriptionSession
impl TranscriptionSession {
    /// Get the input_audio_format
    pub fn get_input_audio_format(&self) -> Option<&String> {
        self.input_audio_format.as_ref()
    }

    /// Set the input_audio_format
    pub fn set_input_audio_format(&mut self, input_audio_format: Option<String>) {
        self.input_audio_format = input_audio_format;
    }

    /// Get the input_audio_noise_reduction
    pub fn get_input_audio_noise_reduction(&self) -> Option<&InputAudioNoiseReduction> {
        self.input_audio_noise_reduction.as_ref()
    }

    /// Set the input_audio_noise_reduction
    pub fn set_input_audio_noise_reduction(
        &mut self,
        input_audio_noise_reduction: Option<InputAudioNoiseReduction>,
    ) {
        self.input_audio_noise_reduction = input_audio_noise_reduction;
    }

    /// Get the modalities
    pub fn get_modalities(&self) -> Option<&Vec<String>> {
        self.modalities.as_ref()
    }

    /// Set the modalities
    pub fn set_modalities(&mut self, modalities: Option<Vec<String>>) {
        self.modalities = modalities;
    }

    /// Get the turn_detection
    pub fn get_turn_detection(&self) -> Option<&TurnDetection> {
        self.turn_detection.as_ref()
    }

    /// Set the turn_detection
    pub fn set_turn_detection(&mut self, turn_detection: Option<TurnDetection>) {
        self.turn_detection = turn_detection;
    }

    /// Serialize to JSON string
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Deserialize from JSON string
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

// Implement getter and setter methods for InputAudioBufferAppendEvent
impl InputAudioBufferAppendEvent {
    /// Get the event_id
    pub fn get_event_id(&self) -> Option<&String> {
        self.event_id.as_ref()
    }

    /// Set the event_id
    pub fn set_event_id(&mut self, event_id: Option<String>) {
        self.event_id = event_id;
    }

    /// Get the client_timestamp
    pub fn get_client_timestamp(&self) -> Option<u64> {
        self.client_timestamp
    }

    /// Set the client_timestamp
    pub fn set_client_timestamp(&mut self, client_timestamp: Option<u64>) {
        self.client_timestamp = client_timestamp;
    }

    /// Get the audio
    pub fn get_audio(&self) -> &str {
        &self.audio
    }

    /// Set the audio
    pub fn set_audio(&mut self, audio: String) {
        self.audio = audio;
    }

    /// Serialize to JSON string
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Deserialize from JSON string
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

/// Event for updating transcription session configuration.
#[derive(Clone, Debug, Serialize, Deserialize, Validate, Default)]
#[serde(default)]
pub struct TranscriptionSessionUpdateEvent {
    /// Unique identifier for the event.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_id: Option<String>,

    /// Client timestamp in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_timestamp: Option<u64>,

    /// Session configuration for transcription.
    pub session: TranscriptionSession,
}

/// Configuration for a transcription session.
#[derive(Clone, Debug, Serialize, Deserialize, Validate, Default)]
#[serde(default)]
pub struct TranscriptionSession {
    /// Input audio format (e.g., "pcm", "wav").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_audio_format: Option<String>,

    /// Input audio noise reduction configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_audio_noise_reduction: Option<InputAudioNoiseReduction>,

    /// Modalities for transcription (text, audio).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modalities: Option<Vec<String>>,

    /// Voice activity detection configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub turn_detection: Option<TurnDetection>,
}

/// Event for appending audio data to the input buffer.
#[derive(Clone, Debug, Serialize, Deserialize, Validate, Default)]
#[serde(default)]
pub struct InputAudioBufferAppendEvent {
    /// Unique identifier for the event.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_id: Option<String>,

    /// Client timestamp in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_timestamp: Option<u64>,

    /// Base64 encoded audio data (wav or pcm).
    pub audio: String,
}

/// Event for appending a video frame to the input buffer.
#[derive(Clone, Debug, Serialize, Deserialize, Validate, Default)]
#[serde(default)]
pub struct InputAudioBufferAppendVideoFrameEvent {
    /// Unique identifier for the event.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_id: Option<String>,

    /// Client timestamp in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_timestamp: Option<u64>,

    /// Base64 encoded JPG image.
    pub video_frame: String,
}

// Implement getter and setter methods for InputAudioBufferAppendVideoFrameEvent
impl InputAudioBufferAppendVideoFrameEvent {
    /// Get the event_id
    pub fn get_event_id(&self) -> Option<&String> {
        self.event_id.as_ref()
    }

    /// Set the event_id
    pub fn set_event_id(&mut self, event_id: Option<String>) {
        self.event_id = event_id;
    }

    /// Get the client_timestamp
    pub fn get_client_timestamp(&self) -> Option<u64> {
        self.client_timestamp
    }

    /// Set the client_timestamp
    pub fn set_client_timestamp(&mut self, client_timestamp: Option<u64>) {
        self.client_timestamp = client_timestamp;
    }

    /// Get the video_frame
    pub fn get_video_frame(&self) -> &str {
        &self.video_frame
    }

    /// Set the video_frame
    pub fn set_video_frame(&mut self, video_frame: String) {
        self.video_frame = video_frame;
    }

    /// Serialize to JSON string
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Deserialize from JSON string
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

/// Event for committing the audio data in the input buffer.
#[derive(Clone, Debug, Serialize, Deserialize, Validate, Default)]
#[serde(default)]
pub struct InputAudioBufferCommitEvent {
    /// Unique identifier for the event.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_id: Option<String>,

    /// Client timestamp in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_timestamp: Option<u64>,
}

// Implement getter and setter methods for InputAudioBufferCommitEvent
impl InputAudioBufferCommitEvent {
    /// Get the event_id
    pub fn get_event_id(&self) -> Option<&String> {
        self.event_id.as_ref()
    }

    /// Set the event_id
    pub fn set_event_id(&mut self, event_id: Option<String>) {
        self.event_id = event_id;
    }

    /// Get the client_timestamp
    pub fn get_client_timestamp(&self) -> Option<u64> {
        self.client_timestamp
    }

    /// Set the client_timestamp
    pub fn set_client_timestamp(&mut self, client_timestamp: Option<u64>) {
        self.client_timestamp = client_timestamp;
    }

    /// Serialize to JSON string
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Deserialize from JSON string
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

/// Event for clearing the audio data in the input buffer.
#[derive(Clone, Debug, Serialize, Deserialize, Validate, Default)]
#[serde(default)]
pub struct InputAudioBufferClearEvent {
    /// Unique identifier for the event.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_id: Option<String>,

    /// Client timestamp in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_timestamp: Option<u64>,
}

// Implement getter and setter methods for InputAudioBufferClearEvent
impl InputAudioBufferClearEvent {
    /// Get the event_id
    pub fn get_event_id(&self) -> Option<&String> {
        self.event_id.as_ref()
    }

    /// Set the event_id
    pub fn set_event_id(&mut self, event_id: Option<String>) {
        self.event_id = event_id;
    }

    /// Get the client_timestamp
    pub fn get_client_timestamp(&self) -> Option<u64> {
        self.client_timestamp
    }

    /// Set the client_timestamp
    pub fn set_client_timestamp(&mut self, client_timestamp: Option<u64>) {
        self.client_timestamp = client_timestamp;
    }

    /// Serialize to JSON string
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Deserialize from JSON string
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

/// Event for creating a new conversation item.
#[derive(Clone, Debug, Serialize, Deserialize, Validate, Default)]
#[serde(default)]
pub struct ConversationItemCreateEvent {
    /// Unique identifier for the event.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_id: Option<String>,

    /// Client timestamp in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_timestamp: Option<u64>,

    /// The conversation item to create.
    pub item: RealtimeConversationItem,
}

// Implement getter and setter methods for ConversationItemCreateEvent
impl ConversationItemCreateEvent {
    /// Get the event_id
    pub fn get_event_id(&self) -> Option<&String> {
        self.event_id.as_ref()
    }

    /// Set the event_id
    pub fn set_event_id(&mut self, event_id: Option<String>) {
        self.event_id = event_id;
    }

    /// Get the client_timestamp
    pub fn get_client_timestamp(&self) -> Option<u64> {
        self.client_timestamp
    }

    /// Set the client_timestamp
    pub fn set_client_timestamp(&mut self, client_timestamp: Option<u64>) {
        self.client_timestamp = client_timestamp;
    }

    /// Get the item
    pub fn get_item(&self) -> &RealtimeConversationItem {
        &self.item
    }

    /// Set the item
    pub fn set_item(&mut self, item: RealtimeConversationItem) {
        self.item = item;
    }

    /// Serialize to JSON string
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Deserialize from JSON string
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

/// Event for deleting a conversation item.
#[derive(Clone, Debug, Serialize, Deserialize, Validate, Default)]
#[serde(default)]
pub struct ConversationItemDeleteEvent {
    /// Unique identifier for the event.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_id: Option<String>,

    /// Client timestamp in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_timestamp: Option<u64>,

    /// ID of the conversation item to delete.
    pub item_id: String,
}

// Implement getter and setter methods for ConversationItemDeleteEvent
impl ConversationItemDeleteEvent {
    /// Get the event_id
    pub fn get_event_id(&self) -> Option<&String> {
        self.event_id.as_ref()
    }

    /// Set the event_id
    pub fn set_event_id(&mut self, event_id: Option<String>) {
        self.event_id = event_id;
    }

    /// Get the client_timestamp
    pub fn get_client_timestamp(&self) -> Option<u64> {
        self.client_timestamp
    }

    /// Set the client_timestamp
    pub fn set_client_timestamp(&mut self, client_timestamp: Option<u64>) {
        self.client_timestamp = client_timestamp;
    }

    /// Serialize to JSON string
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Deserialize from JSON string
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

/// Event for retrieving a conversation item.
#[derive(Clone, Debug, Serialize, Deserialize, Validate, Default)]
#[serde(default)]
pub struct ConversationItemRetrieveEvent {
    /// Unique identifier for the event.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_id: Option<String>,

    /// Client timestamp in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_timestamp: Option<u64>,

    /// ID of the conversation item to retrieve.
    pub item_id: String,
}

// Implement getter and setter methods for ConversationItemRetrieveEvent
impl ConversationItemRetrieveEvent {
    /// Get the event_id
    pub fn get_event_id(&self) -> Option<&String> {
        self.event_id.as_ref()
    }

    /// Set the event_id
    pub fn set_event_id(&mut self, event_id: Option<String>) {
        self.event_id = event_id;
    }

    /// Get the client_timestamp
    pub fn get_client_timestamp(&self) -> Option<u64> {
        self.client_timestamp
    }

    /// Set the client_timestamp
    pub fn set_client_timestamp(&mut self, client_timestamp: Option<u64>) {
        self.client_timestamp = client_timestamp;
    }

    /// Get the item_id
    pub fn get_item_id(&self) -> &str {
        &self.item_id
    }

    /// Set the item_id
    pub fn set_item_id(&mut self, item_id: String) {
        self.item_id = item_id;
    }

    /// Serialize to JSON string
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Deserialize from JSON string
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

/// Event for creating a new response.
#[derive(Clone, Debug, Serialize, Deserialize, Validate, Default)]
#[serde(default)]
pub struct ResponseCreateEvent {
    /// Unique identifier for the event.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_id: Option<String>,

    /// Client timestamp in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_timestamp: Option<u64>,
}

// Implement getter and setter methods for ResponseCreateEvent
impl ResponseCreateEvent {
    /// Get the event_id
    pub fn get_event_id(&self) -> Option<&String> {
        self.event_id.as_ref()
    }

    /// Set the event_id
    pub fn set_event_id(&mut self, event_id: Option<String>) {
        self.event_id = event_id;
    }

    /// Get the client_timestamp
    pub fn get_client_timestamp(&self) -> Option<u64> {
        self.client_timestamp
    }

    /// Set the client_timestamp
    pub fn set_client_timestamp(&mut self, client_timestamp: Option<u64>) {
        self.client_timestamp = client_timestamp;
    }

    /// Serialize to JSON string
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Deserialize from JSON string
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

/// Event for cancelling the current response.
#[derive(Clone, Debug, Serialize, Deserialize, Validate, Default)]
#[serde(default)]
pub struct ResponseCancelEvent {
    /// Unique identifier for the event.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_id: Option<String>,

    /// Client timestamp in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_timestamp: Option<u64>,
}

// Implement getter and setter methods for ResponseCancelEvent
impl ResponseCancelEvent {
    /// Get the event_id
    pub fn get_event_id(&self) -> Option<&String> {
        self.event_id.as_ref()
    }

    /// Set the event_id
    pub fn set_event_id(&mut self, event_id: Option<String>) {
        self.event_id = event_id;
    }

    /// Get the client_timestamp
    pub fn get_client_timestamp(&self) -> Option<u64> {
        self.client_timestamp
    }

    /// Set the client_timestamp
    pub fn set_client_timestamp(&mut self, client_timestamp: Option<u64>) {
        self.client_timestamp = client_timestamp;
    }

    /// Serialize to JSON string
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Deserialize from JSON string
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

// Test module for client events
#[cfg(test)]
mod tests {
    use super::*;
    use crate::real_time::session::Session;
    use crate::real_time::types::{ItemStatus, ItemType, RealtimeConversationItem, Role};

    #[test]
    fn test_client_event_serialization_with_type_field() {
        let mut session = Session::default();
        session.model = Some("glm-realtime".to_string());
        session.modalities = Some(vec!["text".to_string(), "audio".to_string()]);
        session.voice = Some("tongtong".to_string());

        let mut session_update_event = SessionUpdateEvent::default();
        session_update_event.event_id = Some("session-123".to_string());
        session_update_event.client_timestamp = Some(1625097600000);
        session_update_event.set_session(session);

        let client_event = ClientEvent::SessionUpdate(session_update_event);
        let json = serde_json::to_string(&client_event).unwrap();

        // Check if the JSON contains the "type" field with the correct value
        assert!(json.contains("\"type\":\"session.update\""));

        // Parse the JSON to verify it can be deserialized correctly
        let deserialized_event: ClientEvent = serde_json::from_str(&json).unwrap();
        match deserialized_event {
            ClientEvent::SessionUpdate(event) => {
                assert_eq!(event.event_id, Some("session-123".to_string()));
                assert_eq!(event.client_timestamp, Some(1625097600000));
                assert_eq!(event.session.model, Some("glm-realtime".to_string()));
            }
            _ => panic!("Expected SessionUpdate event"),
        }
    }

    #[test]
    fn test_session_update_event_serialization() {
        let mut session = Session::default();
        session.model = Some("glm-realtime".to_string());
        session.modalities = Some(vec!["text".to_string(), "audio".to_string()]);
        session.voice = Some("tongtong".to_string());

        let mut event = SessionUpdateEvent::default();
        event.event_id = Some("session-123".to_string());
        event.client_timestamp = Some(1625097600000);
        event.set_session(session);

        let json = event.to_json().unwrap();
        let deserialized_event: SessionUpdateEvent = SessionUpdateEvent::from_json(&json).unwrap();

        assert_eq!(event.get_event_id(), deserialized_event.get_event_id());
        assert_eq!(
            event.get_client_timestamp(),
            deserialized_event.get_client_timestamp()
        );
        assert_eq!(
            event.get_session().model,
            deserialized_event.get_session().model
        );
    }

    #[test]
    fn test_transcription_session_update_event_serialization() {
        let mut session = TranscriptionSession::default();
        session.input_audio_format = Some("pcm".to_string());
        session.modalities = Some(vec!["text".to_string(), "audio".to_string()]);

        let mut event = TranscriptionSessionUpdateEvent::default();
        event.event_id = Some("transcription-123".to_string());
        event.client_timestamp = Some(1625097600000);
        event.set_session(session);

        let json = event.to_json().unwrap();
        let deserialized_event: TranscriptionSessionUpdateEvent =
            TranscriptionSessionUpdateEvent::from_json(&json).unwrap();

        assert_eq!(event.get_event_id(), deserialized_event.get_event_id());
        assert_eq!(
            event.get_client_timestamp(),
            deserialized_event.get_client_timestamp()
        );
        assert_eq!(
            event.get_session().input_audio_format,
            deserialized_event.get_session().input_audio_format
        );
    }

    #[test]
    fn test_input_audio_buffer_append_event_serialization() {
        let mut event = InputAudioBufferAppendEvent::default();
        event.event_id = Some("audio-123".to_string());
        event.client_timestamp = Some(1625097600000);
        event.set_audio("base64-audio-data".to_string());

        let json = event.to_json().unwrap();
        let deserialized_event: InputAudioBufferAppendEvent =
            InputAudioBufferAppendEvent::from_json(&json).unwrap();

        assert_eq!(event.get_event_id(), deserialized_event.get_event_id());
        assert_eq!(
            event.get_client_timestamp(),
            deserialized_event.get_client_timestamp()
        );
        assert_eq!(event.get_audio(), deserialized_event.get_audio());
    }

    #[test]
    fn test_conversation_item_create_event_serialization() {
        let mut item = RealtimeConversationItem::default();
        item.id = Some("item-123".to_string());
        item.item_type = ItemType::Message;
        item.object = "realtime.item".to_string();
        item.status = Some(ItemStatus::Completed);
        item.role = Some(Role::User);

        let mut event = ConversationItemCreateEvent::default();
        event.event_id = Some("create-123".to_string());
        event.client_timestamp = Some(1625097600000);
        event.set_item(item);

        let json = event.to_json().unwrap();
        let deserialized_event: ConversationItemCreateEvent =
            ConversationItemCreateEvent::from_json(&json).unwrap();

        assert_eq!(event.get_event_id(), deserialized_event.get_event_id());
        assert_eq!(
            event.get_client_timestamp(),
            deserialized_event.get_client_timestamp()
        );
        assert_eq!(event.get_item().id, deserialized_event.get_item().id);
    }

    #[test]
    fn test_input_audio_buffer_append_video_frame_event_serialization() {
        let mut event = InputAudioBufferAppendVideoFrameEvent::default();
        event.event_id = Some("video-123".to_string());
        event.client_timestamp = Some(1625097600000);
        event.set_video_frame("base64-video-data".to_string());

        let json = event.to_json().unwrap();
        let deserialized_event: InputAudioBufferAppendVideoFrameEvent =
            InputAudioBufferAppendVideoFrameEvent::from_json(&json).unwrap();

        assert_eq!(event.get_event_id(), deserialized_event.get_event_id());
        assert_eq!(
            event.get_client_timestamp(),
            deserialized_event.get_client_timestamp()
        );
        assert_eq!(
            event.get_video_frame(),
            deserialized_event.get_video_frame()
        );
    }

    #[test]
    fn test_input_audio_buffer_commit_event_serialization() {
        let mut event = InputAudioBufferCommitEvent::default();
        event.event_id = Some("commit-123".to_string());
        event.client_timestamp = Some(1625097600000);

        let json = event.to_json().unwrap();
        let deserialized_event: InputAudioBufferCommitEvent =
            InputAudioBufferCommitEvent::from_json(&json).unwrap();

        assert_eq!(event.get_event_id(), deserialized_event.get_event_id());
        assert_eq!(
            event.get_client_timestamp(),
            deserialized_event.get_client_timestamp()
        );
    }

    #[test]
    fn test_input_audio_buffer_clear_event_serialization() {
        let mut event = InputAudioBufferClearEvent::default();
        event.event_id = Some("clear-123".to_string());
        event.client_timestamp = Some(1625097600000);

        let json = event.to_json().unwrap();
        let deserialized_event: InputAudioBufferClearEvent =
            InputAudioBufferClearEvent::from_json(&json).unwrap();

        assert_eq!(event.get_event_id(), deserialized_event.get_event_id());
        assert_eq!(
            event.get_client_timestamp(),
            deserialized_event.get_client_timestamp()
        );
    }

    #[test]
    fn test_conversation_item_delete_event_serialization() {
        let mut event = ConversationItemDeleteEvent::default();
        event.event_id = Some("delete-123".to_string());
        event.client_timestamp = Some(1625097600000);
        event.item_id = "item-456".to_string();

        let json = event.to_json().unwrap();
        let deserialized_event: ConversationItemDeleteEvent =
            ConversationItemDeleteEvent::from_json(&json).unwrap();

        assert_eq!(event.get_event_id(), deserialized_event.get_event_id());
        assert_eq!(
            event.get_client_timestamp(),
            deserialized_event.get_client_timestamp()
        );
        assert_eq!(event.item_id, deserialized_event.item_id);
    }

    #[test]
    fn test_conversation_item_retrieve_event_serialization() {
        let mut event = ConversationItemRetrieveEvent::default();
        event.event_id = Some("retrieve-123".to_string());
        event.client_timestamp = Some(1625097600000);
        event.set_item_id("item-789".to_string());

        let json = event.to_json().unwrap();
        let deserialized_event: ConversationItemRetrieveEvent =
            ConversationItemRetrieveEvent::from_json(&json).unwrap();

        assert_eq!(event.get_event_id(), deserialized_event.get_event_id());
        assert_eq!(
            event.get_client_timestamp(),
            deserialized_event.get_client_timestamp()
        );
        assert_eq!(event.get_item_id(), deserialized_event.get_item_id());
    }

    #[test]
    fn test_response_create_event_serialization() {
        let mut event = ResponseCreateEvent::default();
        event.event_id = Some("response-create-123".to_string());
        event.client_timestamp = Some(1625097600000);

        let json = event.to_json().unwrap();
        let deserialized_event: ResponseCreateEvent =
            ResponseCreateEvent::from_json(&json).unwrap();

        assert_eq!(event.get_event_id(), deserialized_event.get_event_id());
        assert_eq!(
            event.get_client_timestamp(),
            deserialized_event.get_client_timestamp()
        );
    }

    #[test]
    fn test_response_cancel_event_serialization() {
        let mut event = ResponseCancelEvent::default();
        event.event_id = Some("response-cancel-123".to_string());
        event.client_timestamp = Some(1625097600000);

        let json = event.to_json().unwrap();
        let deserialized_event: ResponseCancelEvent =
            ResponseCancelEvent::from_json(&json).unwrap();

        assert_eq!(event.get_event_id(), deserialized_event.get_event_id());
        assert_eq!(
            event.get_client_timestamp(),
            deserialized_event.get_client_timestamp()
        );
    }
}
