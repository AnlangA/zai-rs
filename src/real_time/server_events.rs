//! Server-to-client events for GLM-Realtime API.
//!
//! This module defines all event types that can be sent from the server
//! to the client in a real-time conversation.

use serde::{Deserialize, Serialize};
use validator::Validate;

// Import everything from the real_time module to simplify references
use crate::real_time::client_events::TranscriptionSession;
use crate::real_time::session::*;
use crate::real_time::types::*;

/// Represents a server event sent to the client.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ServerEvent {
    /// Error event.
    #[serde(rename = "error")]
    Error(ErrorEvent),
    /// Session created event.
    #[serde(rename = "session.created")]
    SessionCreated(SessionCreatedEvent),
    /// Session updated event.
    #[serde(rename = "session.updated")]
    SessionUpdated(SessionUpdatedEvent),
    /// Transcription session updated event.
    #[serde(rename = "transcription_session.updated")]
    TranscriptionSessionUpdated(TranscriptionSessionUpdatedEvent),
    /// Conversation item created event.
    #[serde(rename = "conversation.item.created")]
    ConversationItemCreated(ConversationItemCreatedEvent),
    /// Conversation item deleted event.
    #[serde(rename = "conversation.item.deleted")]
    ConversationItemDeleted(ConversationItemDeletedEvent),
    /// Conversation item retrieved event.
    #[serde(rename = "conversation.item.retrieved")]
    ConversationItemRetrieved(ConversationItemRetrievedEvent),
    /// Input audio transcription completed event.
    #[serde(rename = "conversation.item.input_audio_transcription.completed")]
    ConversationItemInputAudioTranscriptionCompleted(
        ConversationItemInputAudioTranscriptionCompletedEvent,
    ),
    /// Input audio transcription failed event.
    #[serde(rename = "conversation.item.input_audio_transcription.failed")]
    ConversationItemInputAudioTranscriptionFailed(
        ConversationItemInputAudioTranscriptionFailedEvent,
    ),
    /// Input audio buffer committed event.
    #[serde(rename = "input_audio_buffer.committed")]
    InputAudioBufferCommitted(InputAudioBufferCommittedEvent),
    /// Input audio buffer cleared event.
    #[serde(rename = "input_audio_buffer.cleared")]
    InputAudioBufferCleared(InputAudioBufferClearedEvent),
    /// Input audio buffer speech started event.
    #[serde(rename = "input_audio_buffer.speech_started")]
    InputAudioBufferSpeechStarted(InputAudioBufferSpeechStartedEvent),
    /// Input audio buffer speech stopped event.
    #[serde(rename = "input_audio_buffer.speech_stopped")]
    InputAudioBufferSpeechStopped(InputAudioBufferSpeechStoppedEvent),
    /// Response output item added event.
    #[serde(rename = "response.output_item.added")]
    ResponseOutputItemAdded(ResponseOutputItemAddedEvent),
    /// Response output item done event.
    #[serde(rename = "response.output_item.done")]
    ResponseOutputItemDone(ResponseOutputItemDoneEvent),
    /// Response content part added event.
    #[serde(rename = "response.content_part.added")]
    ResponseContentPartAdded(ResponseContentPartAddedEvent),
    /// Response content part done event.
    #[serde(rename = "response.content_part.done")]
    ResponseContentPartDone(ResponseContentPartDoneEvent),
    /// Response function call arguments done event.
    #[serde(rename = "response.function_call_arguments.done")]
    ResponseFunctionCallArgumentsDone(ResponseFunctionCallArgumentsDoneEvent),
    /// Response function call simple browser event.
    #[serde(rename = "response.function_call.simple_browser")]
    ResponseFunctionCallSimpleBrowser(ResponseFunctionCallSimpleBrowserEvent),
    /// Response text delta event.
    #[serde(rename = "response.text.delta")]
    ResponseTextDelta(ResponseTextDeltaEvent),
    /// Response text done event.
    #[serde(rename = "response.text.done")]
    ResponseTextDone(ResponseTextDoneEvent),
    /// Response audio transcript delta event.
    #[serde(rename = "response.audio_transcript.delta")]
    ResponseAudioTranscriptDelta(ResponseAudioTranscriptDeltaEvent),
    /// Response audio transcript done event.
    #[serde(rename = "response.audio_transcript.done")]
    ResponseAudioTranscriptDone(ResponseAudioTranscriptDoneEvent),
    /// Response audio delta event.
    #[serde(rename = "response.audio.delta")]
    ResponseAudioDelta(ResponseAudioDeltaEvent),
    /// Response audio done event.
    #[serde(rename = "response.audio.done")]
    ResponseAudioDone(ResponseAudioDoneEvent),
    /// Response created event.
    #[serde(rename = "response.created")]
    ResponseCreated(ResponseCreatedEvent),
    /// Response cancelled event.
    #[serde(rename = "response.cancelled")]
    ResponseCancelled(ResponseCancelledEvent),
    /// Response done event.
    #[serde(rename = "response.done")]
    ResponseDone(ResponseDoneEvent),
    /// Rate limits updated event.
    #[serde(rename = "rate_limits.updated")]
    RateLimitsUpdated(RateLimitsUpdatedEvent),
    /// Heartbeat event.
    #[serde(rename = "heartbeat")]
    Heartbeat(HeartbeatEvent),
}

/// Error event.
#[derive(Clone, Debug, Serialize, Deserialize, Validate, Default)]
#[serde(default)]
pub struct ErrorEvent {
    /// Unique identifier for the event.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_id: Option<String>,
    /// Error information.
    pub error: ErrorInfo,
}

// Implement getter methods for ErrorEvent
impl ErrorEvent {
    /// Get the event_id
    pub fn get_event_id(&self) -> Option<&String> {
        self.event_id.as_ref()
    }

    /// Get the error
    pub fn get_error(&self) -> &ErrorInfo {
        &self.error
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

/// Session created event.
#[derive(Clone, Debug, Serialize, Deserialize, Validate, Default)]
#[serde(default)]
pub struct SessionCreatedEvent {
    /// Unique identifier for the event.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_id: Option<String>,

    /// Client timestamp in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_timestamp: Option<u64>,

    /// Session information.
    pub session: Session,
}

/// Session updated event.
#[derive(Clone, Debug, Serialize, Deserialize, Validate, Default)]
#[serde(default)]
pub struct SessionUpdatedEvent {
    /// Unique identifier for the event.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_id: Option<String>,

    /// Client timestamp in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_timestamp: Option<u64>,

    /// Session information.
    pub session: Session,
}

// Implement getter methods for SessionUpdatedEvent
impl SessionUpdatedEvent {
    /// Get the event_id
    pub fn get_event_id(&self) -> Option<&String> {
        self.event_id.as_ref()
    }

    /// Get the client_timestamp
    pub fn get_client_timestamp(&self) -> Option<u64> {
        self.client_timestamp
    }

    /// Get the session
    pub fn get_session(&self) -> &Session {
        &self.session
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

// Implement getter methods for SessionCreatedEvent
impl SessionCreatedEvent {
    /// Get the event_id
    pub fn get_event_id(&self) -> Option<&String> {
        self.event_id.as_ref()
    }

    /// Get the client_timestamp
    pub fn get_client_timestamp(&self) -> Option<u64> {
        self.client_timestamp
    }

    /// Get the session
    pub fn get_session(&self) -> &Session {
        &self.session
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

/// This duplicate definition has been removed.

/// Transcription session updated event.
#[derive(Clone, Debug, Serialize, Deserialize, Validate, Default)]
#[serde(default)]
pub struct TranscriptionSessionUpdatedEvent {
    /// Unique identifier for the event.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_id: Option<String>,

    /// Client timestamp in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_timestamp: Option<u64>,

    /// Transcription session information.
    pub session: TranscriptionSession,
}

// Implement getter methods for TranscriptionSessionUpdatedEvent
impl TranscriptionSessionUpdatedEvent {
    /// Get the event_id
    pub fn get_event_id(&self) -> Option<&String> {
        self.event_id.as_ref()
    }

    /// Get the client_timestamp
    pub fn get_client_timestamp(&self) -> Option<u64> {
        self.client_timestamp
    }

    /// Get the session
    pub fn get_session(&self) -> &TranscriptionSession {
        &self.session
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

/// Conversation item created event.
#[derive(Clone, Debug, Serialize, Deserialize, Validate, Default)]
#[serde(default)]
pub struct ConversationItemCreatedEvent {
    /// Unique identifier for the event.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_id: Option<String>,

    /// Client timestamp in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_timestamp: Option<u64>,

    /// The conversation item that was created.
    pub item: RealtimeConversationItem,
}

// Implement getter methods for ConversationItemCreatedEvent
impl ConversationItemCreatedEvent {
    /// Get the event_id
    pub fn get_event_id(&self) -> Option<&String> {
        self.event_id.as_ref()
    }

    /// Get the client_timestamp
    pub fn get_client_timestamp(&self) -> Option<u64> {
        self.client_timestamp
    }

    /// Get the item
    pub fn get_item(&self) -> &RealtimeConversationItem {
        &self.item
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

/// Conversation item deleted event.
#[derive(Clone, Debug, Serialize, Deserialize, Validate, Default)]
#[serde(default)]
pub struct ConversationItemDeletedEvent {
    /// Unique identifier for the event.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_id: Option<String>,
    /// Client timestamp in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_timestamp: Option<u64>,
    /// ID of the deleted conversation item.
    pub item_id: String,
}

/// Conversation item retrieved event.
#[derive(Clone, Debug, Serialize, Deserialize, Validate, Default)]
#[serde(default)]
pub struct ConversationItemRetrievedEvent {
    /// Unique identifier for the event.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_id: Option<String>,
    /// Client timestamp in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_timestamp: Option<u64>,
    /// The conversation item that was retrieved.
    pub item: RealtimeConversationItem,
}

/// Input audio transcription completed event.
#[derive(Clone, Debug, Serialize, Deserialize, Validate, Default)]
#[serde(default)]
pub struct ConversationItemInputAudioTranscriptionCompletedEvent {
    /// Unique identifier for the event.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_id: Option<String>,
    /// Client timestamp in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_timestamp: Option<u64>,
    /// ID of the conversation item containing the audio.
    pub item_id: String,
    /// Index of the content part containing the audio.
    pub content_index: u32,
    /// Transcription of the audio.
    pub transcript: String,
}

/// Input audio transcription failed event.
#[derive(Clone, Debug, Serialize, Deserialize, Validate, Default)]
#[serde(default)]
pub struct ConversationItemInputAudioTranscriptionFailedEvent {
    /// Unique identifier for the event.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_id: Option<String>,
    /// Client timestamp in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_timestamp: Option<u64>,
    /// ID of the conversation item containing the audio.
    pub item_id: String,
    /// Index of the content part containing the audio.
    pub content_index: u32,
    /// Error information.
    pub error: ErrorInfo,
}

/// Input audio buffer committed event.
#[derive(Clone, Debug, Serialize, Deserialize, Validate, Default)]
#[serde(default)]
pub struct InputAudioBufferCommittedEvent {
    /// Unique identifier for the event.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_id: Option<String>,
    /// Client timestamp in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_timestamp: Option<u64>,
    /// ID of the created user message item.
    pub item_id: String,
}

/// Input audio buffer cleared event.
#[derive(Clone, Debug, Serialize, Deserialize, Validate, Default)]
#[serde(default)]
pub struct InputAudioBufferClearedEvent {
    /// Unique identifier for the event.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_id: Option<String>,
    /// Client timestamp in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_timestamp: Option<u64>,
}

/// Input audio buffer speech started event.
#[derive(Clone, Debug, Serialize, Deserialize, Validate, Default)]
#[serde(default)]
pub struct InputAudioBufferSpeechStartedEvent {
    /// Unique identifier for the event.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_id: Option<String>,
    /// Client timestamp in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_timestamp: Option<u64>,
    /// Milliseconds from session start to when speech was first detected.
    pub audio_start_ms: u32,
    /// ID of the user message item created when speech started.
    pub item_id: String,
}

/// Input audio buffer speech stopped event.
#[derive(Clone, Debug, Serialize, Deserialize, Validate, Default)]
#[serde(default)]
pub struct InputAudioBufferSpeechStoppedEvent {
    /// Unique identifier for the event.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_id: Option<String>,
    /// Client timestamp in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_timestamp: Option<u64>,
    /// Milliseconds from session start to when speech stopped.
    pub audio_end_ms: u32,
    /// ID of the user message item created when speech stopped.
    pub item_id: String,
}

/// Response output item added event.
#[derive(Clone, Debug, Serialize, Deserialize, Validate, Default)]
#[serde(default)]
pub struct ResponseOutputItemAddedEvent {
    /// Unique identifier for the event.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_id: Option<String>,
    /// Client timestamp in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_timestamp: Option<u64>,
    /// ID of the response.
    pub response_id: String,
    /// Index of the output item in the response.
    pub output_index: u32,
    /// The output item that was added.
    pub item: RealtimeConversationItem,
}

/// Response output item done event.
#[derive(Clone, Debug, Serialize, Deserialize, Validate, Default)]
#[serde(default)]
pub struct ResponseOutputItemDoneEvent {
    /// Unique identifier for the event.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_id: Option<String>,
    /// Client timestamp in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_timestamp: Option<u64>,
    /// ID of the response.
    pub response_id: String,
    /// Index of the output item in the response.
    pub output_index: u32,
    /// The output item that was completed.
    pub item: RealtimeConversationItem,
}

/// Response content part added event.
#[derive(Clone, Debug, Serialize, Deserialize, Validate, Default)]
#[serde(default)]
pub struct ResponseContentPartAddedEvent {
    /// Unique identifier for the event.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_id: Option<String>,
    /// Client timestamp in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_timestamp: Option<u64>,
    /// ID of the response.
    pub response_id: String,
    /// ID of the item containing the content.
    pub item_id: String,
    /// Index of the output item in the response.
    pub output_index: u32,
    /// Index of the content part in the item.
    pub content_index: u32,
    /// The content part that was added.
    pub part: crate::real_time::types::ContentPart,
}

/// Response content part done event.
#[derive(Clone, Debug, Serialize, Deserialize, Validate, Default)]
#[serde(default)]
pub struct ResponseContentPartDoneEvent {
    /// Unique identifier for the event.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_id: Option<String>,
    /// Client timestamp in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_timestamp: Option<u64>,
    /// ID of the response.
    pub response_id: String,
    /// ID of the item containing the content.
    pub item_id: String,
    /// Index of the output item in the response.
    pub output_index: u32,
    /// Index of the content part in the item.
    pub content_index: u32,
    /// The content part that was completed.
    pub part: crate::real_time::types::ContentPart,
}

/// Response function call arguments done event.
#[derive(Clone, Debug, Serialize, Deserialize, Validate, Default)]
#[serde(default)]
pub struct ResponseFunctionCallArgumentsDoneEvent {
    /// Unique identifier for the event.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_id: Option<String>,
    /// Client timestamp in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_timestamp: Option<u64>,
    /// ID of the response.
    pub response_id: String,
    /// Index of the output item in the response.
    pub output_index: u32,
    /// Name of the function.
    pub name: String,
    /// Function call arguments as JSON string.
    pub arguments: String,
}

/// Response function call simple browser event.
#[derive(Clone, Debug, Serialize, Deserialize, Validate, Default)]
#[serde(default)]
pub struct ResponseFunctionCallSimpleBrowserEvent {
    /// Unique identifier for the event.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_id: Option<String>,
    /// Client timestamp in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_timestamp: Option<u64>,
    /// Name of the browser tool.
    pub name: String,
    /// Session information.
    pub session: SimpleBrowserSession,
}

/// Session information for simple browser event.
#[derive(Clone, Debug, Serialize, Deserialize, Validate, Default)]
#[serde(default)]
pub struct SimpleBrowserSession {
    /// Beta fields containing browser information.
    pub beta_fields: BetaFields,
}

/// Response text delta event.
#[derive(Clone, Debug, Serialize, Deserialize, Validate, Default)]
#[serde(default)]
pub struct ResponseTextDeltaEvent {
    /// Unique identifier for the event.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_id: Option<String>,
    /// Client timestamp in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_timestamp: Option<u64>,
    /// ID of the response.
    pub response_id: String,
    /// ID of the item containing the text.
    pub item_id: String,
    /// Index of the output item in the response.
    pub output_index: u32,
    /// Index of the content part in the item.
    pub content_index: u32,
    /// The delta text.
    pub delta: String,
}

/// Response text done event.
#[derive(Clone, Debug, Serialize, Deserialize, Validate, Default)]
#[serde(default)]
pub struct ResponseTextDoneEvent {
    /// Unique identifier for the event.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_id: Option<String>,
    /// Client timestamp in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_timestamp: Option<u64>,
    /// ID of the response.
    pub response_id: String,
    /// ID of the item containing the text.
    pub item_id: String,
    /// Index of the output item in the response.
    pub output_index: u32,
    /// Index of the content part in the item.
    pub content_index: u32,
    /// The final complete text.
    pub text: String,
}

/// Response audio transcript delta event.
#[derive(Clone, Debug, Serialize, Deserialize, Validate, Default)]
#[serde(default)]
pub struct ResponseAudioTranscriptDeltaEvent {
    /// Unique identifier for the event.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_id: Option<String>,
    /// Client timestamp in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_timestamp: Option<u64>,
    /// ID of the response.
    pub response_id: String,
    /// ID of the item containing the audio.
    pub item_id: String,
    /// Index of the output item in the response.
    pub output_index: u32,
    /// Index of the content part in the item.
    pub content_index: u32,
    /// The delta transcript.
    pub delta: String,
}

/// Response audio transcript done event.
#[derive(Clone, Debug, Serialize, Deserialize, Validate, Default)]
#[serde(default)]
pub struct ResponseAudioTranscriptDoneEvent {
    /// Unique identifier for the event.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_id: Option<String>,
    /// Client timestamp in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_timestamp: Option<u64>,
    /// ID of the response.
    pub response_id: String,
    /// ID of the item containing the audio.
    pub item_id: String,
    /// Index of the output item in the response.
    pub output_index: u32,
    /// Index of the content part in the item.
    pub content_index: u32,
    /// The final complete transcript.
    pub transcript: String,
}

/// Response audio delta event.
#[derive(Clone, Debug, Serialize, Deserialize, Validate, Default)]
#[serde(default)]
pub struct ResponseAudioDeltaEvent {
    /// Unique identifier for the event.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_id: Option<String>,
    /// Client timestamp in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_timestamp: Option<u64>,
    /// ID of the response.
    pub response_id: String,
    /// ID of the item containing the audio.
    pub item_id: String,
    /// Index of the output item in the response.
    pub output_index: u32,
    /// Index of the content part in the item.
    pub content_index: u32,
    /// Base64 encoded audio data.
    pub delta: String,
}

/// Response audio done event.
#[derive(Clone, Debug, Serialize, Deserialize, Validate, Default)]
#[serde(default)]
pub struct ResponseAudioDoneEvent {
    /// Unique identifier for the event.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_id: Option<String>,
    /// Client timestamp in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_timestamp: Option<u64>,
    /// ID of the response.
    pub response_id: String,
    /// ID of the item containing the audio.
    pub item_id: String,
    /// Index of the output item in the response.
    pub output_index: u32,
    /// Index of the content part in the item.
    pub content_index: u32,
}

/// Response created event.
#[derive(Clone, Debug, Serialize, Deserialize, Validate, Default)]
#[serde(default)]
pub struct ResponseCreatedEvent {
    /// Unique identifier for the event.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_id: Option<String>,
    /// Client timestamp in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_timestamp: Option<u64>,
    /// The response that was created.
    pub response: RealtimeResponse,
}

/// Response cancelled event.
#[derive(Clone, Debug, Serialize, Deserialize, Validate, Default)]
#[serde(default)]
pub struct ResponseCancelledEvent {
    /// Unique identifier for the event.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_id: Option<String>,
    /// Client timestamp in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_timestamp: Option<u64>,
    /// The response that was cancelled.
    pub response: RealtimeResponse,
}

/// Response done event.
#[derive(Clone, Debug, Serialize, Deserialize, Validate, Default)]
#[serde(default)]
pub struct ResponseDoneEvent {
    /// Unique identifier for the event.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_id: Option<String>,
    /// Client timestamp in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_timestamp: Option<u64>,
    /// The response that was completed.
    pub response: RealtimeResponse,
}

/// Rate limits updated event.
#[derive(Clone, Debug, Serialize, Deserialize, Validate, Default)]
#[serde(default)]
pub struct RateLimitsUpdatedEvent {
    /// Unique identifier for the event.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_id: Option<String>,
    /// Client timestamp in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_timestamp: Option<u64>,
    /// List of rate limit information.
    pub rate_limits: Vec<RateLimit>,
}

/// Heartbeat event.
#[derive(Clone, Debug, Serialize, Deserialize, Validate, Default)]
#[serde(default)]
pub struct HeartbeatEvent {
    /// Unique identifier for the event.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_id: Option<String>,
    /// Client timestamp in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_timestamp: Option<u64>,
}

// Test module for server events
#[cfg(test)]
mod tests {
    use super::*;
    use crate::real_time::session::Session;
    use crate::real_time::types::{ItemStatus, ItemType, RealtimeConversationItem};

    #[test]
    fn test_server_event_serialization_with_type_field() {
        let mut session = Session::default();
        session.model = Some("glm-realtime".to_string());
        session.modalities = Some(vec!["text".to_string(), "audio".to_string()]);
        session.voice = Some("tongtong".to_string());

        let mut session_created_event = SessionCreatedEvent::default();
        session_created_event.event_id = Some("event123".to_string());
        session_created_event.client_timestamp = Some(1625097600000);
        session_created_event.session = session;

        let server_event = ServerEvent::SessionCreated(session_created_event);
        let json = serde_json::to_string(&server_event).unwrap();

        // Check if the JSON contains the "type" field with the correct value
        assert!(json.contains("\"type\":\"session.created\""));

        // Parse the JSON to verify it can be deserialized correctly
        let deserialized_event: ServerEvent = serde_json::from_str(&json).unwrap();
        match deserialized_event {
            ServerEvent::SessionCreated(event) => {
                assert_eq!(event.event_id, Some("event123".to_string()));
                assert_eq!(event.client_timestamp, Some(1625097600000));
                assert_eq!(event.session.model, Some("glm-realtime".to_string()));
            }
            _ => panic!("Expected SessionCreated event"),
        }
    }

    #[test]
    fn test_transcription_session_updated_event_serialization() {
        let mut session = TranscriptionSession::default();
        session.input_audio_format = Some("pcm".to_string());
        session.modalities = Some(vec!["text".to_string(), "audio".to_string()]);

        let mut event = TranscriptionSessionUpdatedEvent::default();
        event.event_id = Some("event123".to_string());
        event.client_timestamp = Some(1625097600000);
        event.session = session;

        let server_event = ServerEvent::TranscriptionSessionUpdated(event);
        let json = serde_json::to_string(&server_event).unwrap();

        // Check if the JSON contains the "type" field with the correct value
        assert!(json.contains("\"type\":\"transcription_session.updated\""));
    }

    #[test]
    fn test_session_created_event_serialization() {
        let mut session = Session::default();
        session.model = Some("glm-realtime".to_string());
        session.modalities = Some(vec!["text".to_string(), "audio".to_string()]);
        session.voice = Some("tongtong".to_string());

        let mut event = SessionCreatedEvent::default();
        event.event_id = Some("session-123".to_string());
        event.client_timestamp = Some(1625097600000);
        event.session = session;

        let json = event.to_json().unwrap();
        let deserialized_event: SessionCreatedEvent =
            SessionCreatedEvent::from_json(&json).unwrap();

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
    fn test_session_updated_event_serialization() {
        let mut session = Session::default();
        session.model = Some("glm-realtime".to_string());
        session.modalities = Some(vec!["text".to_string(), "audio".to_string()]);
        session.voice = Some("tongtong".to_string());

        let mut event = SessionUpdatedEvent::default();
        event.event_id = Some("session-123".to_string());
        event.client_timestamp = Some(1625097600000);
        event.session = session;

        let json = event.to_json().unwrap();
        let deserialized_event: SessionUpdatedEvent =
            SessionUpdatedEvent::from_json(&json).unwrap();

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
    fn test_error_event_serialization() {
        let mut error_info = ErrorInfo::default();
        error_info.message = Some("Failed to process request".to_string());

        let mut event = ErrorEvent::default();
        event.event_id = Some("error-123".to_string());
        event.error = error_info;

        let json = event.to_json().unwrap();
        let deserialized_event: ErrorEvent = ErrorEvent::from_json(&json).unwrap();

        assert_eq!(event.get_event_id(), deserialized_event.get_event_id());
        assert_eq!(
            event.get_error().get_message(),
            deserialized_event.get_error().get_message()
        );
    }

    #[test]
    fn test_conversation_item_created_event_serialization() {
        let mut item = RealtimeConversationItem::default();
        item.id = Some("item-123".to_string());
        item.item_type = ItemType::Message;
        item.object = "realtime.item".to_string();
        item.status = Some(ItemStatus::Completed);

        let mut event = ConversationItemCreatedEvent::default();
        event.event_id = Some("item-created-123".to_string());
        event.client_timestamp = Some(1625097600000);
        event.item = item;

        let json = event.to_json().unwrap();
        let deserialized_event: ConversationItemCreatedEvent =
            ConversationItemCreatedEvent::from_json(&json).unwrap();

        assert_eq!(event.get_event_id(), deserialized_event.get_event_id());
        assert_eq!(
            event.get_client_timestamp(),
            deserialized_event.get_client_timestamp()
        );
        assert_eq!(event.get_item().id, deserialized_event.get_item().id);
        assert_eq!(
            event.get_item().item_type,
            deserialized_event.get_item().item_type
        );
    }
}
