//! Data structures from the official GLM-Realtime protocol.
//!
//! Mirrors `RealtimeConversationItem`, `RealtimeResponse`, the `session.update`
//! payload, and supporting enums. Wire shapes are pinned to the repository's
//! `spec/upstream/asyncapi-2026-07-11.json` snapshot.

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
///
/// The tuning fields apply to server-side VAD. Leave them unset for
/// client-driven VAD, where the caller explicitly commits audio and creates a
/// response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnDetection {
    /// VAD strategy.
    #[serde(rename = "type")]
    pub type_: TurnDetectionType,
    /// Automatically create a response when server VAD detects the end of a
    /// turn.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub create_response: Option<bool>,
    /// Interrupt an in-progress response when server VAD detects new speech.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interrupt_response: Option<bool>,
    /// Audio retained before detected speech, in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prefix_padding_ms: Option<u32>,
    /// Silence required to end a turn, in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub silence_duration_ms: Option<u32>,
    /// VAD activation threshold in the inclusive range `0.0..=1.0`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub threshold: Option<f64>,
}

impl TurnDetection {
    /// Create a `turn_detection` object with the given VAD strategy.
    pub fn new(type_: TurnDetectionType) -> Self {
        Self {
            type_,
            create_response: None,
            interrupt_response: None,
            prefix_padding_ms: None,
            silence_duration_ms: None,
            threshold: None,
        }
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

/// Output modality requested from the model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RealtimeModality {
    /// Generate text output.
    Text,
    /// Generate audio output.
    Audio,
}

/// Built-in voice used for audio output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum RealtimeVoice {
    /// Default Tongtong voice.
    #[default]
    #[serde(rename = "tongtong")]
    Tongtong,
    /// General-purpose male voice.
    #[serde(rename = "xiaochen")]
    Xiaochen,
    /// Tianmei female voice.
    #[serde(rename = "female-tianmei")]
    FemaleTianmei,
    /// Young male voice.
    #[serde(rename = "male-qn-daxuesheng")]
    MaleQnDaxuesheng,
    /// Professional male voice.
    #[serde(rename = "male-qn-jingying")]
    MaleQnJingying,
    /// Lovely-girl voice.
    #[serde(rename = "lovely_girl")]
    LovelyGirl,
    /// Young female voice.
    #[serde(rename = "female-shaonv")]
    FemaleShaonv,
}

/// Microphone placement used by input-audio noise reduction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NoiseReductionType {
    /// A microphone close to the speaker.
    NearField,
    /// A microphone farther from the speaker.
    FarField,
}

/// `input_audio_noise_reduction` inside `session.update`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct InputAudioNoiseReduction {
    /// Noise-reduction profile.
    #[serde(rename = "type")]
    pub type_: NoiseReductionType,
}

impl InputAudioNoiseReduction {
    /// Select a noise-reduction profile.
    pub fn new(type_: NoiseReductionType) -> Self {
        Self { type_ }
    }
}

/// Optional server-generated greeting configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GreetingConfig {
    /// Whether the greeting is enabled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable: Option<bool>,
    /// Greeting text supplied to the model.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
}

/// `beta_fields` object inside `session.update`.
#[derive(Debug, Clone, Serialize, Deserialize)]
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

impl Default for BetaFields {
    fn default() -> Self {
        Self {
            chat_mode: Some(ChatMode::Audio),
            tts_source: None,
            auto_search: None,
        }
    }
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
    #[serde(default = "empty_json_object")]
    pub parameters: serde_json::Value,
}

fn empty_json_object() -> serde_json::Value {
    serde_json::Value::Object(serde_json::Map::new())
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
    /// Model id selected for this session. [`SessionBuilder`](super::SessionBuilder)
    /// fills this from the type-safe model passed to
    /// [`RealtimeClient::session`](super::RealtimeClient::session).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Input audio format (`wav`, `pcm16`, or `pcm24`).
    pub input_audio_format: InputAudioFormat,
    /// Output audio format. The current protocol accepts only `pcm`.
    pub output_audio_format: OutputAudioFormat,
    /// System instructions guiding the model.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,
    /// Output modalities. The protocol default is text plus audio.
    #[serde(default = "default_modalities")]
    pub modalities: Vec<RealtimeModality>,
    /// Voice used when audio output is requested.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice: Option<RealtimeVoice>,
    /// Sampling temperature in the inclusive range `0.0..=1.0`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    /// Maximum number of response text tokens (`1..=1024`). `None` uses the
    /// server default (`"inf"`, currently equivalent to 1024).
    ///
    /// The upstream schema represents this numeric limit as a JSON string, so
    /// custom serde preserves that exact wire shape while callers use a `u16`.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "optional_u16_string"
    )]
    pub max_response_output_tokens: Option<u16>,
    /// VAD strategy.
    pub turn_detection: TurnDetection,
    /// Optional microphone noise-reduction profile.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_audio_noise_reduction: Option<InputAudioNoiseReduction>,
    /// Beta / mode toggles.
    #[serde(default)]
    pub beta_fields: BetaFields,
    /// Optional greeting generated when the session starts.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub greeting_config: Option<GreetingConfig>,
    /// Function tools advertised to the model.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<RealtimeTool>,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            model: None,
            input_audio_format: InputAudioFormat::default(),
            output_audio_format: OutputAudioFormat::default(),
            instructions: None,
            modalities: default_modalities(),
            voice: None,
            temperature: None,
            max_response_output_tokens: None,
            turn_detection: TurnDetection::new(TurnDetectionType::default()),
            input_audio_noise_reduction: None,
            beta_fields: BetaFields::default(),
            greeting_config: None,
            tools: Vec::new(),
        }
    }
}

fn default_modalities() -> Vec<RealtimeModality> {
    vec![RealtimeModality::Text, RealtimeModality::Audio]
}

mod optional_u16_string {
    use serde::{Deserialize as _, Deserializer, Serializer, de::Error as _};

    pub(super) fn serialize<S>(value: &Option<u16>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match value {
            Some(value) => serializer.serialize_some(&value.to_string()),
            None => serializer.serialize_none(),
        }
    }

    pub(super) fn deserialize<'de, D>(deserializer: D) -> Result<Option<u16>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Option::<String>::deserialize(deserializer)?;
        value
            .map(|value| {
                if value == "inf" {
                    return Ok(None);
                }
                value
                    .parse::<u16>()
                    .map_err(|_| {
                        D::Error::custom(
                            "max_response_output_tokens must be \"inf\" or a decimal u16 string",
                        )
                    })
                    .map(Some)
            })
            .transpose()
            .map(Option::flatten)
    }
}

/// `item.type` discriminator inside [`RealtimeConversationItem`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemType {
    /// A chat message (with `role` + `content`).
    Message,
    /// A function/tool call emitted by the model.
    FunctionCall,
    /// The caller's reply to a function/tool call.
    FunctionCallOutput,
}

/// One content part within a conversation item message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemContent {
    /// Content type: `input_audio`, `input_text`, `text`.
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub type_: Option<String>,
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
                type_: Some("input_text".to_string()),
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
    /// Number of text tokens.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_tokens: Option<u64>,
    /// Number of audio tokens.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio_tokens: Option<u64>,
    /// Number of cached tokens.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cached_tokens: Option<u64>,
}

/// `RealtimeResponse.usage`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RealtimeUsage {
    /// Total tokens for this response.
    #[serde(default)]
    pub total_tokens: u64,
    /// Input tokens.
    #[serde(default)]
    pub input_tokens: u64,
    /// Output tokens.
    #[serde(default)]
    pub output_tokens: u64,
    /// Input-token breakdown.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_token_details: Option<TokenDetails>,
    /// Output-token breakdown.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_token_details: Option<TokenDetails>,
}

/// `RealtimeResponse`: emitted by `response.created` / `response.done`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealtimeResponse {
    /// Response id.
    pub id: String,
    /// Always `"realtime.response"`.
    pub object: String,
    /// Response status (`in_progress`, `completed`, `cancelled`, `failed`, or
    /// `incomplete`).
    pub status: String,
    /// Token usage (present on `response.done`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<RealtimeUsage>,
}
