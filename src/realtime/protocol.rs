//! Data structures from the official GLM-Realtime protocol.
//!
//! Mirrors `RealtimeConversationItem`, `RealtimeResponse`, the `session.update`
//! payload, and supporting enums. See:
//! <https://github.com/MetaGLM/glm-realtime-sdk/blob/main/GLM-Realtime-doc-for-llm.md>

use serde::{Deserialize, Serialize};

use super::audio::{InputAudioFormat, OutputAudioFormat};

/// VAD (voice-activity-detection) mode. `ClientVad` (default) lets the client
/// decide when to commit audio; `ServerVad` has the server detect speech and
/// auto-commit (and supports interruption handling).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TurnDetectionType {
    /// Client-driven VAD (client uploads + commits audio manually).
    #[default]
    ClientVad,
    /// Server-driven VAD (server detects speech, auto-commits, handles
    /// interruption via `response.cancel`).
    ServerVad,
}

/// `turn_detection` object inside `session.update`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnDetection {
    /// VAD strategy.
    #[serde(rename = "type")]
    pub type_: TurnDetectionType,
}

impl TurnDetection {
    pub fn new(type_: TurnDetectionType) -> Self {
        Self { type_ }
    }
}

/// Conversation mode under `beta_fields.chat_mode`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChatMode {
    /// Passive video: video frames are sent alongside audio.
    VideoPassive,
    /// Audio-only conversation (default).
    #[default]
    Audio,
}

/// `beta_fields` object inside `session.update`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BetaFields {
    /// Conversation mode.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chat_mode: Option<ChatMode>,
    /// TTS source, e.g. `"e2e"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tts_source: Option<String>,
    /// Enable the server-side built-in web search (audio mode only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_search: Option<bool>,
}

/// A function tool advertised to the model via `session.update.tools`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealtimeTool {
    /// Always `"function"`.
    #[serde(rename = "type")]
    pub type_: String,
    /// Function name.
    pub name: String,
    /// Human-readable description the model uses to decide when to call.
    pub description: String,
    /// JSON Schema describing accepted parameters.
    pub parameters: serde_json::Value,
}

impl RealtimeTool {
    /// Build a function tool from a name + description + JSON-Schema
    /// parameters.
    pub fn function(
        name: impl Into<String>,
        description: impl Into<String>,
        parameters: serde_json::Value,
    ) -> Self {
        Self {
            type_: "function".to_string(),
            name: name.into(),
            description: description.into(),
            parameters,
        }
    }
}

/// The inner `session` object carried by the `session.update` client event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    /// Input audio format (`wav` / `wav48`).
    pub input_audio_format: InputAudioFormat,
    /// Output audio format (`pcm` / `mp3`).
    pub output_audio_format: OutputAudioFormat,
    /// System instructions guiding the model.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,
    /// VAD strategy.
    pub turn_detection: TurnDetection,
    /// Beta / mode toggles.
    pub beta_fields: BetaFields,
    /// Function tools (ServerVAD requires re-sending `turn_detection`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<RealtimeTool>,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            input_audio_format: InputAudioFormat::default(),
            output_audio_format: OutputAudioFormat::default(),
            instructions: None,
            turn_detection: TurnDetection::new(TurnDetectionType::default()),
            beta_fields: BetaFields::default(),
            tools: Vec::new(),
        }
    }
}

/// `item.type` discriminator inside [`RealtimeConversationItem`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemType {
    Message,
    FunctionCall,
    FunctionCallOutput,
}

/// One content part within a conversation item message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemContent {
    /// Content type: `input_audio`, `input_text`, `text`.
    #[serde(rename = "type")]
    pub type_: String,
    /// Text content (`input_text` / `text`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// Base64 audio (`input_audio`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio: Option<String>,
    /// Audio transcript.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transcript: Option<String>,
}

/// `RealtimeConversationItem`: a message, function call, or function-call
/// output inserted via `conversation.item.create`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealtimeConversationItem {
    /// Item id (client- or server-generated).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Item kind.
    #[serde(rename = "type")]
    pub type_: ItemType,
    /// Always `"realtime.item"`.
    pub object: String,
    /// `completed` / `incomplete`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    /// Sender role (`message` only): `user` / `assistant` / `system`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    /// Message content parts (`message` only).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub content: Vec<ItemContent>,
    /// Function name (`function_call` only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Function arguments (`function_call` only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<String>,
    /// Function output (`function_call_output` only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
}

impl RealtimeConversationItem {
    /// A user text message: `conversation.item.create` for textual input.
    pub fn user_text(text: impl Into<String>) -> Self {
        Self {
            id: None,
            type_: ItemType::Message,
            object: "realtime.item".to_string(),
            status: Some("completed".to_string()),
            role: Some("user".to_string()),
            content: vec![ItemContent {
                type_: "input_text".to_string(),
                text: Some(text.into()),
                audio: None,
                transcript: None,
            }],
            name: None,
            arguments: None,
            output: None,
        }
    }

    /// A function-call output to feed back to the model.
    pub fn function_output(call_name: impl Into<String>, output: impl Into<String>) -> Self {
        Self {
            id: None,
            type_: ItemType::FunctionCallOutput,
            object: "realtime.item".to_string(),
            status: None,
            role: None,
            content: Vec::new(),
            name: Some(call_name.into()),
            arguments: None,
            output: Some(output.into()),
        }
    }
}

/// Token-usage detail breakdown.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenDetails {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cached_tokens: Option<u64>,
}

/// `RealtimeResponse.usage`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RealtimeUsage {
    #[serde(default)]
    pub total_tokens: u64,
    #[serde(default)]
    pub input_tokens: u64,
    #[serde(default)]
    pub output_tokens: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_token_details: Option<TokenDetails>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_token_details: Option<TokenDetails>,
}

/// `RealtimeResponse`: emitted by `response.created` / `response.done`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealtimeResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Always `"realtime.response"`.
    pub object: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<RealtimeUsage>,
}
