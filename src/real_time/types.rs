//! # Real-time API Data Types
//!
//! This module contains all data structures, enums, and types needed for
//! the GLM-Realtime API. It defines both client and server events,
//! conversation items, session configurations, and other related types.

use serde::{Deserialize, Serialize};
// No longer needed - removing unused import

/// The different types of voices available for the model
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Voice {
    /// Universal male voice
    Xiaochen,
    /// Universal female voice (default)
    Tongtong,
    /// Sweet female voice
    FemaleTianmei,
    /// Young girl voice
    FemaleShaonv,
    /// Young male university student voice
    MaleQnDaxuesheng,
    /// Elite young male voice
    MaleQnJingying,
    /// Cute little girl voice
    LovelyGirl,
}

impl Default for Voice {
    fn default() -> Self {
        Voice::Tongtong
    }
}

impl ToString for Voice {
    fn to_string(&self) -> String {
        match self {
            Voice::Xiaochen => "xiaochen".to_string(),
            Voice::Tongtong => "tongtong".to_string(),
            Voice::FemaleTianmei => "female-tianmei".to_string(),
            Voice::FemaleShaonv => "female-shaonv".to_string(),
            Voice::MaleQnDaxuesheng => "male-qn-daxuesheng".to_string(),
            Voice::MaleQnJingying => "male-qn-jingying".to_string(),
            Voice::LovelyGirl => "lovely_girl".to_string(),
        }
    }
}

/// Chat mode for the real-time conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChatMode {
    /// Audio only mode
    Audio,
    /// Video passive mode
    VideoPassive,
}

impl Default for ChatMode {
    fn default() -> Self {
        ChatMode::Audio
    }
}

impl ToString for ChatMode {
    fn to_string(&self) -> String {
        match self {
            ChatMode::Audio => "audio".to_string(),
            ChatMode::VideoPassive => "video_passive".to_string(),
        }
    }
}

/// Input audio noise reduction type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NoiseReductionType {
    /// For close-distance microphones like headsets
    NearField,
    /// For far-distance microphones like laptops or conference room mics
    FarField,
}

impl Default for NoiseReductionType {
    fn default() -> Self {
        NoiseReductionType::FarField
    }
}

/// VAD (Voice Activity Detection) type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VadType {
    /// Server-side VAD
    ServerVad,
}

/// Configuration for noise reduction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoiseReductionConfig {
    #[serde(rename = "type")]
    pub reduction_type: NoiseReductionType,
}

impl Default for NoiseReductionConfig {
    fn default() -> Self {
        Self {
            reduction_type: NoiseReductionType::FarField,
        }
    }
}

/// Configuration for VAD (Voice Activity Detection)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VadConfig {
    #[serde(rename = "type")]
    pub vad_type: VadType,
    /// Whether to automatically create a response when VAD stop event occurs
    pub create_response: Option<bool>,
    /// Whether to automatically interrupt any ongoing response when VAD start event occurs
    pub interrupt_response: Option<bool>,
    /// Amount of audio to include before VAD detects speech (in milliseconds)
    pub prefix_padding_ms: Option<u32>,
    /// Silence duration for detecting speech stop (in milliseconds)
    pub silence_duration_ms: Option<u32>,
    /// VAD activation threshold (0.0 to 1.0)
    pub threshold: Option<f32>,
}

/// Tool definition for function calling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    #[serde(rename = "type")]
    pub tool_type: String,
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

/// Greeting configuration for the session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GreetingConfig {
    /// Whether to enable greeting
    pub enable: Option<bool>,
    /// Custom greeting content (max 1024 characters)
    pub content: Option<String>,
}

/// Beta fields for the session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BetaFields {
    /// Chat mode (audio or video)
    pub chat_mode: String,
    /// Text-to-speech source (e.g., "e2e")
    pub tts_source: Option<String>,
    /// Whether to enable auto search
    pub auto_search: Option<bool>,
    /// Greeting configuration
    pub greeting_config: Option<GreetingConfig>,
}

/// Session configuration for real-time conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    /// Model name (e.g., "glm-realtime-flash")
    pub model: Option<String>,
    /// Modalities to control model output (text, audio)
    pub modalities: Option<Vec<String>>,
    /// System instructions for the model
    pub instructions: Option<String>,
    /// Voice to use for audio output
    pub voice: Option<String>,
    /// Input audio format (wav, pcm)
    pub input_audio_format: String,
    /// Output audio format (pcm)
    pub output_audio_format: String,
    /// Input audio noise reduction configuration
    pub input_audio_noise_reduction: Option<NoiseReductionConfig>,
    /// VAD configuration
    pub turn_detection: Option<VadConfig>,
    /// Model temperature (0.0 to 1.0)
    pub temperature: Option<f32>,
    /// Maximum response output tokens
    pub max_response_output_tokens: Option<String>,
    /// Tools for function calling
    pub tools: Option<Vec<Tool>>,
    /// Beta fields
    pub beta_fields: Option<BetaFields>,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            model: Some("glm-realtime".to_string()),
            modalities: Some(vec!["text".to_string(), "audio".to_string()]),
            instructions: None,
            voice: Some("tongtong".to_string()),
            input_audio_format: "wav".to_string(),
            output_audio_format: "pcm".to_string(),
            input_audio_noise_reduction: Some(NoiseReductionConfig::default()),
            turn_detection: None,
            temperature: Some(0.7),
            max_response_output_tokens: Some("inf".to_string()),
            tools: None,
            beta_fields: Some(BetaFields {
                chat_mode: ChatMode::default().to_string(),
                tts_source: None,
                auto_search: Some(false),
                greeting_config: None,
            }),
        }
    }
}

/// Content types for conversation items
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContentType {
    /// Input audio content
    InputAudio,
    /// Input text content
    InputText,
    /// Text content
    Text,
    /// Audio content
    Audio,
}

/// Content part of a conversation item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentPart {
    #[serde(rename = "type")]
    pub content_type: ContentType,
    /// Text content (for text types)
    pub text: Option<String>,
    /// Audio content (base64 encoded, for audio types)
    pub audio: Option<String>,
    /// Audio transcript (for audio types)
    pub transcript: Option<String>,
}

/// Types of conversation items
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemType {
    /// Message item
    Message,
    /// Function call item
    FunctionCall,
    /// Function call output item
    FunctionCallOutput,
}

/// Roles for message items
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemRole {
    /// User role
    User,
    /// Assistant role
    Assistant,
    /// System role
    System,
}

/// Status of a conversation item
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemStatus {
    /// Item is completed
    Completed,
    /// Item is incomplete
    Incomplete,
    /// Item is in progress
    InProgress,
}

/// A conversation item in the real-time session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealtimeConversationItem {
    /// Unique ID of the item
    pub id: Option<String>,
    /// Type of the item
    #[serde(rename = "type")]
    pub item_type: ItemType,
    /// Object type (always "realtime.item")
    pub object: String,
    /// Status of the item
    pub status: Option<ItemStatus>,
    /// Role of the message sender (for message items)
    pub role: Option<ItemRole>,
    /// Content array of the item
    pub content: Option<Vec<ContentPart>>,
    /// Function name (for function call items)
    pub name: Option<String>,
    /// Function arguments (for function call items)
    pub arguments: Option<String>,
    /// Function output (for function call output items)
    pub output: Option<String>,
}

/// Token usage details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenDetails {
    /// Cached tokens
    pub cached_tokens: Option<u32>,
    /// Text tokens
    pub text_tokens: Option<u32>,
    /// Audio tokens
    pub audio_tokens: Option<u32>,
}

/// Usage statistics for a response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseUsage {
    /// Total tokens used
    pub total_tokens: u32,
    /// Input tokens used
    pub input_tokens: u32,
    /// Output tokens used
    pub output_tokens: u32,
    /// Input token details
    pub input_token_details: Option<TokenDetails>,
    /// Output token details
    pub output_token_details: Option<TokenDetails>,
}

/// Status of a response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResponseStatus {
    /// Response is in progress
    InProgress,
    /// Response is completed
    Completed,
    /// Response is cancelled
    Cancelled,
}

/// A response from the real-time model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealtimeResponse {
    /// Unique ID of the response
    pub id: String,
    /// Object type (always "realtime.response")
    pub object: String,
    /// Status of the response
    pub status: ResponseStatus,
    /// Usage statistics
    pub usage: Option<ResponseUsage>,
}

/// Rate limit information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimit {
    /// Name of the rate limit
    pub name: String,
    /// Maximum limit
    pub limit: u32,
    /// Remaining requests
    pub remaining: u32,
    /// Seconds until reset
    pub reset_seconds: f32,
}

/// Error details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorDetail {
    /// Error type
    #[serde(rename = "type")]
    pub error_type: String,
    /// Error code
    pub code: String,
    /// Error message
    pub message: String,
}

// Client Events

/// Base for all client events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseClientEvent {
    /// Unique ID of the event
    pub event_id: Option<String>,
    /// Client timestamp (milliseconds)
    pub client_timestamp: Option<u64>,
    /// Type of the event
    #[serde(rename = "type")]
    pub event_type: String,
}

/// Event to update session configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionUpdateEvent {
    /// Base event properties
    #[serde(flatten)]
    pub base: BaseClientEvent,
    /// Session configuration
    pub session: SessionConfig,
}

/// Event to update transcription session configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionSessionUpdateEvent {
    /// Base event properties
    #[serde(flatten)]
    pub base: BaseClientEvent,
    /// Session configuration
    pub session: SessionConfig,
}

/// Event to append audio to the input buffer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputAudioBufferAppendEvent {
    /// Base event properties
    #[serde(flatten)]
    pub base: BaseClientEvent,
    /// Audio data (base64 encoded)
    pub audio: String,
}

/// Event to append a video frame to the input buffer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputAudioBufferAppendVideoFrameEvent {
    /// Base event properties
    #[serde(flatten)]
    pub base: BaseClientEvent,
    /// Video frame data (base64 encoded jpg)
    pub video_frame: String,
}

/// Event to commit the audio buffer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputAudioBufferCommitEvent {
    /// Base event properties
    #[serde(flatten)]
    pub base: BaseClientEvent,
}

/// Event to clear the audio buffer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputAudioBufferClearEvent {
    /// Base event properties
    #[serde(flatten)]
    pub base: BaseClientEvent,
}

/// Event to create a conversation item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationItemCreateEvent {
    /// Base event properties
    #[serde(flatten)]
    pub base: BaseClientEvent,
    /// Item to create
    pub item: RealtimeConversationItem,
}

/// Event to delete a conversation item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationItemDeleteEvent {
    /// Base event properties
    #[serde(flatten)]
    pub base: BaseClientEvent,
    /// ID of the item to delete
    pub item_id: String,
}

/// Event to retrieve a conversation item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationItemRetrieveEvent {
    /// Base event properties
    #[serde(flatten)]
    pub base: BaseClientEvent,
    /// ID of the item to retrieve
    pub item_id: String,
}

/// Event to create a response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseCreateEvent {
    /// Base event properties
    #[serde(flatten)]
    pub base: BaseClientEvent,
}

/// Event to cancel a response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseCancelEvent {
    /// Base event properties
    #[serde(flatten)]
    pub base: BaseClientEvent,
}

/// Enum representing all possible client events
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientEvent {
    /// Update session configuration
    #[serde(rename = "session.update")]
    SessionUpdate(SessionUpdateEvent),
    /// Update transcription session configuration
    #[serde(rename = "transcription_session.update")]
    TranscriptionSessionUpdate(TranscriptionSessionUpdateEvent),
    /// Append audio to input buffer
    #[serde(rename = "input_audio_buffer.append")]
    InputAudioBufferAppend(InputAudioBufferAppendEvent),
    /// Append video frame to input buffer
    #[serde(rename = "input_audio_buffer.append_video_frame")]
    InputAudioBufferAppendVideoFrame(InputAudioBufferAppendVideoFrameEvent),
    /// Commit audio buffer
    #[serde(rename = "input_audio_buffer.commit")]
    InputAudioBufferCommit(InputAudioBufferCommitEvent),
    /// Clear audio buffer
    #[serde(rename = "input_audio_buffer.clear")]
    InputAudioBufferClear(InputAudioBufferClearEvent),
    /// Create conversation item
    #[serde(rename = "conversation.item.create")]
    ConversationItemCreate(ConversationItemCreateEvent),
    /// Delete conversation item
    #[serde(rename = "conversation.item.delete")]
    ConversationItemDelete(ConversationItemDeleteEvent),
    /// Retrieve conversation item
    #[serde(rename = "conversation.item.retrieve")]
    ConversationItemRetrieve(ConversationItemRetrieveEvent),
    /// Create response
    #[serde(rename = "response.create")]
    ResponseCreate(ResponseCreateEvent),
    /// Cancel response
    #[serde(rename = "response.cancel")]
    ResponseCancel(ResponseCancelEvent),
}

// Server Events

/// Base for all server events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseServerEvent {
    /// Unique ID of the event
    pub event_id: String,
    /// Type of the event
    #[serde(rename = "type")]
    pub event_type: String,
    /// Client timestamp (milliseconds)
    pub client_timestamp: Option<u64>,
}

/// Event indicating an error occurred
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorEvent {
    /// Base event properties
    #[serde(flatten)]
    pub base: BaseServerEvent,
    /// Error details
    pub error: ErrorDetail,
}

/// Event indicating the session was created
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionCreatedEvent {
    /// Base event properties
    #[serde(flatten)]
    pub base: BaseServerEvent,
    /// Session information
    pub session: SessionInfo,
}

/// Event indicating the session was updated
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionUpdatedEvent {
    /// Base event properties
    #[serde(flatten)]
    pub base: BaseServerEvent,
    /// Session information
    pub session: SessionInfo,
}

/// Event indicating the transcription session was updated
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionSessionUpdatedEvent {
    /// Base event properties
    #[serde(flatten)]
    pub base: BaseServerEvent,
    /// Session information
    pub session: SessionInfo,
}

/// Event indicating a conversation item was created
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationItemCreatedEvent {
    /// Base event properties
    #[serde(flatten)]
    pub base: BaseServerEvent,
    /// The created item
    pub item: RealtimeConversationItem,
}

/// Event indicating a conversation item was deleted
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationItemDeletedEvent {
    /// Base event properties
    #[serde(flatten)]
    pub base: BaseServerEvent,
    /// ID of the deleted item
    pub item_id: String,
}

/// Event indicating a conversation item was retrieved
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationItemRetrievedEvent {
    /// Base event properties
    #[serde(flatten)]
    pub base: BaseServerEvent,
    /// The retrieved item
    pub item: RealtimeConversationItem,
}

/// Event indicating input audio transcription completed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputAudioTranscriptionCompletedEvent {
    /// Base event properties
    #[serde(flatten)]
    pub base: BaseServerEvent,
    /// ID of the item
    pub item_id: String,
    /// Content index
    pub content_index: u32,
    /// Transcribed text
    pub transcript: String,
}

/// Event indicating input audio transcription failed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputAudioTranscriptionFailedEvent {
    /// Base event properties
    #[serde(flatten)]
    pub base: BaseServerEvent,
    /// ID of the item
    pub item_id: String,
    /// Content index
    pub content_index: u32,
    /// Error details
    pub error: ErrorDetail,
}

/// Event indicating the input audio buffer was committed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputAudioBufferCommittedEvent {
    /// Base event properties
    #[serde(flatten)]
    pub base: BaseServerEvent,
    /// ID of the created item
    pub item_id: String,
}

/// Event indicating the input audio buffer was cleared
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputAudioBufferClearedEvent {
    /// Base event properties
    #[serde(flatten)]
    pub base: BaseServerEvent,
}

/// Event indicating speech was detected in the audio buffer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputAudioBufferSpeechStartedEvent {
    /// Base event properties
    #[serde(flatten)]
    pub base: BaseServerEvent,
    /// Audio start time in milliseconds
    pub audio_start_ms: u32,
    /// ID of the item
    pub item_id: String,
}

/// Event indicating speech stopped in the audio buffer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputAudioBufferSpeechStoppedEvent {
    /// Base event properties
    #[serde(flatten)]
    pub base: BaseServerEvent,
    /// Audio end time in milliseconds
    pub audio_end_ms: u32,
    /// ID of the item
    pub item_id: String,
}

/// Event indicating an output item was added to a response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseOutputItemAddedEvent {
    /// Base event properties
    #[serde(flatten)]
    pub base: BaseServerEvent,
    /// ID of the response
    pub response_id: String,
    /// Output index
    pub output_index: u32,
    /// The added item
    pub item: RealtimeConversationItem,
}

/// Event indicating an output item was marked as done
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseOutputItemDoneEvent {
    /// Base event properties
    #[serde(flatten)]
    pub base: BaseServerEvent,
    /// ID of the response
    pub response_id: String,
    /// Output index
    pub output_index: u32,
    /// The item
    pub item: RealtimeConversationItem,
}

/// Event indicating a content part was added to a response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseContentPartAddedEvent {
    /// Base event properties
    #[serde(flatten)]
    pub base: BaseServerEvent,
    /// ID of the response
    pub response_id: String,
    /// ID of the item
    pub item_id: String,
    /// Output index
    pub output_index: u32,
    /// Content index
    pub content_index: u32,
    /// The added part
    pub part: ContentPart,
}

/// Event indicating a content part was marked as done
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseContentPartDoneEvent {
    /// Base event properties
    #[serde(flatten)]
    pub base: BaseServerEvent,
    /// ID of the response
    pub response_id: String,
    /// ID of the item
    pub item_id: String,
    /// Output index
    pub output_index: u32,
    /// Content index
    pub content_index: u32,
    /// The part
    pub part: ContentPart,
}

/// Event indicating function call arguments are done
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseFunctionCallArgumentsDoneEvent {
    /// Base event properties
    #[serde(flatten)]
    pub base: BaseServerEvent,
    /// ID of the response
    pub response_id: String,
    /// Output index
    pub output_index: u32,
    /// Function name
    pub name: String,
    /// Function arguments (JSON string)
    pub arguments: String,
}

/// Event indicating a function call to the simple browser
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseFunctionCallSimpleBrowserEvent {
    /// Base event properties
    #[serde(flatten)]
    pub base: BaseServerEvent,
    /// Function name
    pub name: String,
    /// Session information
    pub session: BrowserSessionInfo,
}

/// Event indicating a text delta in a response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseTextDeltaEvent {
    /// Base event properties
    #[serde(flatten)]
    pub base: BaseServerEvent,
    /// ID of the response
    pub response_id: String,
    /// ID of the item
    pub item_id: String,
    /// Output index
    pub output_index: u32,
    /// Content index
    pub content_index: u32,
    /// Text delta
    pub delta: String,
}

/// Event indicating text in a response is done
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseTextDoneEvent {
    /// Base event properties
    #[serde(flatten)]
    pub base: BaseServerEvent,
    /// ID of the response
    pub response_id: String,
    /// ID of the item
    pub item_id: String,
    /// Output index
    pub output_index: u32,
    /// Content index
    pub content_index: u32,
    /// Final text
    pub text: String,
}

/// Event indicating an audio transcript delta in a response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseAudioTranscriptDeltaEvent {
    /// Base event properties
    #[serde(flatten)]
    pub base: BaseServerEvent,
    /// ID of the response
    pub response_id: String,
    /// ID of the item
    pub item_id: String,
    /// Output index
    pub output_index: u32,
    /// Content index
    pub content_index: u32,
    /// Transcript delta
    pub delta: String,
}

/// Event indicating an audio transcript in a response is done
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseAudioTranscriptDoneEvent {
    /// Base event properties
    #[serde(flatten)]
    pub base: BaseServerEvent,
    /// ID of the response
    pub response_id: String,
    /// ID of the item
    pub item_id: String,
    /// Output index
    pub output_index: u32,
    /// Content index
    pub content_index: u32,
    /// Final transcript
    pub transcript: String,
}

/// Event indicating an audio delta in a response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseAudioDeltaEvent {
    /// Base event properties
    #[serde(flatten)]
    pub base: BaseServerEvent,
    /// ID of the response
    pub response_id: String,
    /// ID of the item
    pub item_id: String,
    /// Output index
    pub output_index: u32,
    /// Content index
    pub content_index: u32,
    /// Audio delta (base64 encoded)
    pub delta: String,
}

/// Event indicating audio in a response is done
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseAudioDoneEvent {
    /// Base event properties
    #[serde(flatten)]
    pub base: BaseServerEvent,
    /// ID of the response
    pub response_id: String,
    /// ID of the item
    pub item_id: String,
    /// Output index
    pub output_index: u32,
    /// Content index
    pub content_index: u32,
}

/// Event indicating a response was created
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseCreatedEvent {
    /// Base event properties
    #[serde(flatten)]
    pub base: BaseServerEvent,
    /// The response
    pub response: RealtimeResponse,
}

/// Event indicating a response was cancelled
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseCancelledEvent {
    /// Base event properties
    #[serde(flatten)]
    pub base: BaseServerEvent,
    /// The response
    pub response: RealtimeResponse,
}

/// Event indicating a response is done
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseDoneEvent {
    /// Base event properties
    #[serde(flatten)]
    pub base: BaseServerEvent,
    /// The response
    pub response: RealtimeResponse,
}

/// Event indicating rate limits were updated
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitsUpdatedEvent {
    /// Base event properties
    #[serde(flatten)]
    pub base: BaseServerEvent,
    /// Rate limit information
    pub rate_limits: Vec<RateLimit>,
}

/// Event indicating a heartbeat (connection is alive)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatEvent {
    /// Base event properties
    #[serde(flatten)]
    pub base: BaseServerEvent,
}

/// Session information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    /// Object type (always "realtime.session")
    pub object: String,
    /// Session ID
    pub id: String,
    /// Model name
    pub model: String,
    /// Modalities
    pub modalities: Option<Vec<String>>,
    /// Instructions
    pub instructions: Option<String>,
    /// Voice
    pub voice: Option<String>,
    /// Input audio format
    pub input_audio_format: Option<String>,
    /// Output audio format
    pub output_audio_format: Option<String>,
    /// Temperature
    pub temperature: Option<f32>,
    /// VAD configuration
    pub turn_detection: Option<VadConfig>,
    /// Tools
    pub tools: Option<Vec<Tool>>,
    /// Beta fields
    pub beta_fields: Option<BetaFields>,
}

/// Browser session information for function calls
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserSessionInfo {
    /// Beta fields
    pub beta_fields: BrowserBetaFields,
}

/// Browser beta fields
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserBetaFields {
    /// Simple browser information
    pub simple_browser: SimpleBrowserInfo,
}

/// Simple browser information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimpleBrowserInfo {
    /// Description
    pub description: String,
    /// Search metadata
    pub search_meta: String,
    /// Additional metadata
    pub meta: String,
    /// Text citation
    pub text_citation: String,
}

/// Enum representing all possible server events
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ServerEvent {
    /// Error occurred
    #[serde(rename = "error")]
    Error(ErrorEvent),
    /// Session created
    #[serde(rename = "session.created")]
    SessionCreated(SessionCreatedEvent),
    /// Session updated
    #[serde(rename = "session.updated")]
    SessionUpdated(SessionUpdatedEvent),
    /// Transcription session updated
    #[serde(rename = "transcription_session.updated")]
    TranscriptionSessionUpdated(TranscriptionSessionUpdatedEvent),
    /// Conversation item created
    #[serde(rename = "conversation.item.created")]
    ConversationItemCreated(ConversationItemCreatedEvent),
    /// Conversation item deleted
    #[serde(rename = "conversation.item.deleted")]
    ConversationItemDeleted(ConversationItemDeletedEvent),
    /// Conversation item retrieved
    #[serde(rename = "conversation.item.retrieved")]
    ConversationItemRetrieved(ConversationItemRetrievedEvent),
    /// Input audio transcription completed
    #[serde(rename = "conversation.item.input_audio_transcription.completed")]
    InputAudioTranscriptionCompleted(InputAudioTranscriptionCompletedEvent),
    /// Input audio transcription failed
    #[serde(rename = "conversation.item.input_audio_transcription.failed")]
    InputAudioTranscriptionFailed(InputAudioTranscriptionFailedEvent),
    /// Input audio buffer committed
    #[serde(rename = "input_audio_buffer.committed")]
    InputAudioBufferCommitted(InputAudioBufferCommittedEvent),
    /// Input audio buffer cleared
    #[serde(rename = "input_audio_buffer.cleared")]
    InputAudioBufferCleared(InputAudioBufferClearedEvent),
    /// Speech started in input audio buffer
    #[serde(rename = "input_audio_buffer.speech_started")]
    InputAudioBufferSpeechStarted(InputAudioBufferSpeechStartedEvent),
    /// Speech stopped in input audio buffer
    #[serde(rename = "input_audio_buffer.speech_stopped")]
    InputAudioBufferSpeechStopped(InputAudioBufferSpeechStoppedEvent),
    /// Response output item added
    #[serde(rename = "response.output_item.added")]
    ResponseOutputItemAdded(ResponseOutputItemAddedEvent),
    /// Response output item done
    #[serde(rename = "response.output_item.done")]
    ResponseOutputItemDone(ResponseOutputItemDoneEvent),
    /// Response content part added
    #[serde(rename = "response.content_part.added")]
    ResponseContentPartAdded(ResponseContentPartAddedEvent),
    /// Response content part done
    #[serde(rename = "response.content_part.done")]
    ResponseContentPartDone(ResponseContentPartDoneEvent),
    /// Response function call arguments done
    #[serde(rename = "response.function_call_arguments.done")]
    ResponseFunctionCallArgumentsDone(ResponseFunctionCallArgumentsDoneEvent),
    /// Response function call simple browser
    #[serde(rename = "response.function_call.simple_browser")]
    ResponseFunctionCallSimpleBrowser(ResponseFunctionCallSimpleBrowserEvent),
    /// Response text delta
    #[serde(rename = "response.text.delta")]
    ResponseTextDelta(ResponseTextDeltaEvent),
    /// Response text done
    #[serde(rename = "response.text.done")]
    ResponseTextDone(ResponseTextDoneEvent),
    /// Response audio transcript delta
    #[serde(rename = "response.audio_transcript.delta")]
    ResponseAudioTranscriptDelta(ResponseAudioTranscriptDeltaEvent),
    /// Response audio transcript done
    #[serde(rename = "response.audio_transcript.done")]
    ResponseAudioTranscriptDone(ResponseAudioTranscriptDoneEvent),
    /// Response audio delta
    #[serde(rename = "response.audio.delta")]
    ResponseAudioDelta(ResponseAudioDeltaEvent),
    /// Response audio done
    #[serde(rename = "response.audio.done")]
    ResponseAudioDone(ResponseAudioDoneEvent),
    /// Response created
    #[serde(rename = "response.created")]
    ResponseCreated(ResponseCreatedEvent),
    /// Response cancelled
    #[serde(rename = "response.cancelled")]
    ResponseCancelled(ResponseCancelledEvent),
    /// Response done
    #[serde(rename = "response.done")]
    ResponseDone(ResponseDoneEvent),
    /// Rate limits updated
    #[serde(rename = "rate_limits.updated")]
    RateLimitsUpdated(RateLimitsUpdatedEvent),
    /// Heartbeat
    #[serde(rename = "heartbeat")]
    Heartbeat(HeartbeatEvent),
}

/// Trait for handling server events
pub trait EventHandler {
    /// Handle an error event
    fn on_error(&mut self, event: ErrorEvent);

    /// Handle a session created event
    fn on_session_created(&mut self, event: SessionCreatedEvent);

    /// Handle a session updated event
    fn on_session_updated(&mut self, event: SessionUpdatedEvent);

    /// Handle a response text delta event
    fn on_response_text_delta(&mut self, event: ResponseTextDeltaEvent);

    /// Handle a response text done event
    fn on_response_text_done(&mut self, event: ResponseTextDoneEvent);

    /// Handle a response audio delta event
    fn on_response_audio_delta(&mut self, event: ResponseAudioDeltaEvent);

    /// Handle a response audio done event
    fn on_response_audio_done(&mut self, event: ResponseAudioDoneEvent);

    /// Handle a response done event
    fn on_response_done(&mut self, event: ResponseDoneEvent);

    /// Handle a heartbeat event
    fn on_heartbeat(&mut self, event: HeartbeatEvent);

    /// Handle unknown or unimplemented events
    fn on_unknown_event(&mut self, event: serde_json::Value);
}

/// Default event handler that logs events
#[derive(Debug, Clone)]
pub struct DefaultEventHandler;

impl EventHandler for DefaultEventHandler {
    fn on_error(&mut self, event: ErrorEvent) {
        log::error!("Realtime error: {:?}", event);
    }

    fn on_session_created(&mut self, event: SessionCreatedEvent) {
        log::info!("Session created: {:?}", event.session.id);
    }

    fn on_session_updated(&mut self, event: SessionUpdatedEvent) {
        log::info!("Session updated: {:?}", event.session.id);
    }

    fn on_response_text_delta(&mut self, event: ResponseTextDeltaEvent) {
        log::info!("Text delta: {}", event.delta);
    }

    fn on_response_text_done(&mut self, event: ResponseTextDoneEvent) {
        log::info!("Text done: {}", event.text);
    }

    fn on_response_audio_delta(&mut self, event: ResponseAudioDeltaEvent) {
        log::debug!("Received audio delta of {} bytes", event.delta.len());
    }

    fn on_response_audio_done(&mut self, _event: ResponseAudioDoneEvent) {
        log::debug!("Audio stream done");
    }

    fn on_response_done(&mut self, event: ResponseDoneEvent) {
        log::info!("Response done: {:?}", event.response);
    }

    fn on_heartbeat(&mut self, _event: HeartbeatEvent) {
        log::debug!("Received heartbeat");
    }

    fn on_unknown_event(&mut self, event: serde_json::Value) {
        log::warn!("Unknown event: {:?}", event);
    }
}

/// Adapter to bridge EventHandler with WebSocketEventHandler
pub struct EventHandlerAdapter {
    /// The real-time event handler
    event_handler: Box<dyn EventHandler + Send + Sync>,
}

impl EventHandlerAdapter {
    /// Create a new adapter with the given event handler
    pub fn new(event_handler: Box<dyn EventHandler + Send + Sync>) -> Self {
        Self { event_handler }
    }

    /// Parse and handle a server message
    async fn handle_server_message(
        &mut self,
        message: &str,
    ) -> Result<(), crate::client::error::ZaiError> {
        use crate::client::error::ZaiError;

        // Try to parse as a known server event
        match serde_json::from_str::<ServerEvent>(message) {
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
                        // For other events, pass as JSON value
                        log::debug!("Unhandled server event: {:?}", event);
                    }
                }
            }
            Err(e) => {
                // If parsing fails, try to parse as a generic JSON value
                match serde_json::from_str::<serde_json::Value>(message) {
                    Ok(value) => {
                        log::warn!(
                            "Failed to parse as known event: {}. Treating as unknown event.",
                            e
                        );
                        self.event_handler.on_unknown_event(value);
                    }
                    Err(e) => {
                        log::error!("Failed to parse message as JSON: {}", e);
                        return Err(ZaiError::Unknown {
                            code: 0,
                            message: format!("Failed to parse message as JSON: {}", e),
                        });
                    }
                }
            }
        }

        Ok(())
    }
}

impl crate::client::wss::WebSocketEventHandler for EventHandlerAdapter {
    fn handle_text_message(&mut self, message: &str) -> Result<(), crate::client::error::ZaiError> {
        // Use tokio::spawn to make the async function work in a sync context
        // This is a common pattern when bridging async and sync code
        let message = message.to_string();

        // Block on the async function
        match futures::executor::block_on(self.handle_server_message(&message)) {
            Ok(_) => Ok(()),
            Err(e) => Err(e),
        }
    }

    fn handle_binary_message(&mut self, data: &[u8]) -> Result<(), crate::client::error::ZaiError> {
        log::debug!("Received binary data of length: {}", data.len());
        // For binary data, we don't have a specific handler in the real-time API
        // So we just log it
        Ok(())
    }

    fn handle_ping(&mut self, data: &[u8]) -> Result<(), crate::client::error::ZaiError> {
        log::debug!("Received ping with data length: {}", data.len());
        Ok(())
    }

    fn handle_pong(&mut self, data: &[u8]) -> Result<(), crate::client::error::ZaiError> {
        log::debug!("Received pong with data length: {}", data.len());
        Ok(())
    }

    fn handle_close(
        &mut self,
        frame: Option<tokio_tungstenite::tungstenite::protocol::CloseFrame<'static>>,
    ) -> Result<(), crate::client::error::ZaiError> {
        log::info!("WebSocket close frame received: {:?}", frame);
        Ok(())
    }

    fn handle_error(&mut self, error: String) -> Result<(), crate::client::error::ZaiError> {
        log::error!("WebSocket error: {}", error);
        // Create an error event and pass it to the event handler
        let error_event = ErrorEvent {
            base: BaseServerEvent {
                event_id: crate::client::wss::generate_event_id(),
                event_type: "error".to_string(),
                client_timestamp: Some(crate::client::wss::get_current_timestamp()),
            },
            error: ErrorDetail {
                error_type: "websocket_error".to_string(),
                code: "0".to_string(),
                message: error,
            },
        };
        self.event_handler.on_error(error_event);
        Ok(())
    }
}
