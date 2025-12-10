//! Session configuration and management for GLM-Realtime API.
//!
//! This module defines structures related to session configuration, including
//! model settings, audio/video options, and other parameters that control
//! the behavior of the real-time conversation.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use validator::Validate;

/// Configuration for a real-time session.
#[derive(Clone, Debug, Serialize, Deserialize, Validate, Default)]
#[serde(default)]
pub struct Session {
    /// Model name (e.g., "glm-realtime", "glm-realtime-flash", "glm-realtime-air").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    /// Modalities to use (text, audio, etc.).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modalities: Option<Vec<String>>,

    /// System instructions for the model.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,

    /// Voice to use for audio output.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice: Option<String>,

    /// Input audio format.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_audio_format: Option<String>,

    /// Output audio format.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_audio_format: Option<String>,

    /// Input audio noise reduction configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_audio_noise_reduction: Option<InputAudioNoiseReduction>,

    /// Voice activity detection configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub turn_detection: Option<TurnDetection>,

    /// Model temperature (0.0 to 1.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,

    /// Maximum response output tokens.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_response_output_tokens: Option<String>,

    /// Tools available for function calling.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,

    /// Beta configuration fields.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub beta_fields: Option<BetaFields>,
}

// Implement getter methods for Session
impl Session {
    /// Get the model
    pub fn get_model(&self) -> Option<&String> {
        self.model.as_ref()
    }

    /// Get the modalities
    pub fn get_modalities(&self) -> Option<&Vec<String>> {
        self.modalities.as_ref()
    }

    /// Get the instructions
    pub fn get_instructions(&self) -> Option<&String> {
        self.instructions.as_ref()
    }

    /// Get the voice
    pub fn get_voice(&self) -> Option<&String> {
        self.voice.as_ref()
    }

    /// Get the input_audio_format
    pub fn get_input_audio_format(&self) -> Option<&String> {
        self.input_audio_format.as_ref()
    }

    /// Get the output_audio_format
    pub fn get_output_audio_format(&self) -> Option<&String> {
        self.output_audio_format.as_ref()
    }

    /// Get the input_audio_noise_reduction
    pub fn get_input_audio_noise_reduction(&self) -> Option<&InputAudioNoiseReduction> {
        self.input_audio_noise_reduction.as_ref()
    }

    /// Get the turn_detection
    pub fn get_turn_detection(&self) -> Option<&TurnDetection> {
        self.turn_detection.as_ref()
    }

    /// Get the temperature
    pub fn get_temperature(&self) -> Option<f32> {
        self.temperature
    }

    /// Get the max_response_output_tokens
    pub fn get_max_response_output_tokens(&self) -> Option<&String> {
        self.max_response_output_tokens.as_ref()
    }

    /// Get the tools
    pub fn get_tools(&self) -> Option<&Vec<Tool>> {
        self.tools.as_ref()
    }

    /// Get the beta_fields
    pub fn get_beta_fields(&self) -> Option<&BetaFields> {
        self.beta_fields.as_ref()
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

/// Configuration for input audio noise reduction.
#[derive(Clone, Debug, Serialize, Deserialize, Validate, Default)]
#[serde(default)]
pub struct InputAudioNoiseReduction {
    /// Type of noise reduction.
    pub r#type: String,
}

// Implement getter methods for InputAudioNoiseReduction
impl InputAudioNoiseReduction {
    /// Get the type
    pub fn get_type(&self) -> &str {
        &self.r#type
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

/// Configuration for voice activity detection (VAD).
#[derive(Clone, Debug, Serialize, Deserialize, Validate, Default)]
#[serde(default)]
pub struct TurnDetection {
    /// Type of VAD detection.
    pub r#type: String,

    /// Whether to create response when VAD stops.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub create_response: Option<bool>,

    /// Whether to interrupt response when VAD starts.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interrupt_response: Option<bool>,

    /// Only for ServerVAD mode: padding in milliseconds before VAD detection.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prefix_padding_ms: Option<u32>,

    /// Only for ServerVAD mode: silence duration in milliseconds for speech stop detection.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub silence_duration_ms: Option<u32>,

    /// Only for ServerVAD mode: threshold for VAD activation (0.0 to 1.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub threshold: Option<f32>,
}

// Implement getter methods for TurnDetection
impl TurnDetection {
    /// Get the type
    pub fn get_type(&self) -> &str {
        &self.r#type
    }

    /// Get the create_response
    pub fn get_create_response(&self) -> Option<bool> {
        self.create_response
    }

    /// Get the interrupt_response
    pub fn get_interrupt_response(&self) -> Option<bool> {
        self.interrupt_response
    }

    /// Get the prefix_padding_ms
    pub fn get_prefix_padding_ms(&self) -> Option<u32> {
        self.prefix_padding_ms
    }

    /// Get the silence_duration_ms
    pub fn get_silence_duration_ms(&self) -> Option<u32> {
        self.silence_duration_ms
    }

    /// Get the threshold
    pub fn get_threshold(&self) -> Option<f32> {
        self.threshold
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

/// Definition of a tool for function calling.
#[derive(Clone, Debug, Serialize, Deserialize, Validate, Default)]
#[serde(default)]
pub struct Tool {
    /// Type of the tool (e.g., "function").
    pub r#type: String,

    /// Name of the function.
    pub name: String,

    /// Description of what the function does.
    pub description: String,

    /// JSON schema for function parameters.
    pub parameters: ToolParameters,
}

// Implement getter methods for Tool
impl Tool {
    /// Get the type
    pub fn get_type(&self) -> &str {
        &self.r#type
    }

    /// Get the name
    pub fn get_name(&self) -> &str {
        &self.name
    }

    /// Get the description
    pub fn get_description(&self) -> &str {
        &self.description
    }

    /// Get the parameters
    pub fn get_parameters(&self) -> &ToolParameters {
        &self.parameters
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

/// Parameters for a function tool.
#[derive(Clone, Debug, Serialize, Deserialize, Validate, Default)]
#[serde(default)]
pub struct ToolParameters {
    /// Type of parameters (usually "object").
    pub r#type: String,

    /// Property definitions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<HashMap<String, serde_json::Value>>,

    /// Required parameters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<Vec<String>>,
}

// Implement getter methods for ToolParameters
impl ToolParameters {
    /// Get the type
    pub fn get_type(&self) -> &str {
        &self.r#type
    }

    /// Get the properties
    pub fn get_properties(&self) -> Option<&HashMap<String, serde_json::Value>> {
        self.properties.as_ref()
    }

    /// Get the required
    pub fn get_required(&self) -> Option<&Vec<String>> {
        self.required.as_ref()
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

/// Beta configuration fields.
#[derive(Clone, Debug, Serialize, Deserialize, Validate, Default)]
#[serde(default)]
pub struct BetaFields {
    /// Chat mode (e.g., "video_passive", "audio").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chat_mode: Option<String>,

    /// Text-to-speech source.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tts_source: Option<String>,

    /// Whether to enable auto search.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_search: Option<bool>,

    /// Greeting configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub greeting_config: Option<GreetingConfig>,

    /// Simple browser configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub simple_browser: Option<SimpleBrowser>,
}

// Implement getter methods for BetaFields
impl BetaFields {
    /// Get the chat_mode
    pub fn get_chat_mode(&self) -> Option<&String> {
        self.chat_mode.as_ref()
    }

    /// Get the tts_source
    pub fn get_tts_source(&self) -> Option<&String> {
        self.tts_source.as_ref()
    }

    /// Get the auto_search
    pub fn get_auto_search(&self) -> Option<bool> {
        self.auto_search
    }

    /// Get the greeting_config
    pub fn get_greeting_config(&self) -> Option<&GreetingConfig> {
        self.greeting_config.as_ref()
    }

    /// Get the simple_browser
    pub fn get_simple_browser(&self) -> Option<&SimpleBrowser> {
        self.simple_browser.as_ref()
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

/// Configuration for greeting/welcome message.
#[derive(Clone, Debug, Serialize, Deserialize, Validate, Default)]
#[serde(default)]
pub struct GreetingConfig {
    /// Whether to enable greeting.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable: Option<bool>,

    /// Custom greeting content.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
}

// Implement getter methods for GreetingConfig
impl GreetingConfig {
    /// Get the enable
    pub fn get_enable(&self) -> Option<bool> {
        self.enable
    }

    /// Get the content
    pub fn get_content(&self) -> Option<&String> {
        self.content.as_ref()
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

/// Simple browser configuration for web search.
#[derive(Clone, Debug, Serialize, Deserialize, Validate, Default)]
#[serde(default)]
pub struct SimpleBrowser {
    /// Description including delay tactics.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Search metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_meta: Option<String>,

    /// Additional metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<String>,

    /// Text citation information.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_citation: Option<String>,
}

// Implement getter methods for SimpleBrowser
impl SimpleBrowser {
    /// Get the description
    pub fn get_description(&self) -> Option<&String> {
        self.description.as_ref()
    }

    /// Get the search_meta
    pub fn get_search_meta(&self) -> Option<&String> {
        self.search_meta.as_ref()
    }

    /// Get the meta
    pub fn get_meta(&self) -> Option<&String> {
        self.meta.as_ref()
    }

    /// Get the text_citation
    pub fn get_text_citation(&self) -> Option<&String> {
        self.text_citation.as_ref()
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

// Test module for session
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn test_session_serialization() {
        let mut session = Session::default();
        session.model = Some("glm-realtime".to_string());
        session.modalities = Some(vec!["text".to_string(), "audio".to_string()]);
        session.voice = Some("tongtong".to_string());

        let json = session.to_json().unwrap();
        let deserialized_session: Session = Session::from_json(&json).unwrap();

        assert_eq!(session.get_model(), deserialized_session.get_model());
        assert_eq!(
            session.get_modalities(),
            deserialized_session.get_modalities()
        );
        assert_eq!(session.get_voice(), deserialized_session.get_voice());
    }

    #[test]
    fn test_turn_detection_serialization() {
        let mut turn_detection = TurnDetection::default();
        turn_detection.r#type = "server_vad".to_string();
        turn_detection.threshold = Some(0.5);

        let json = turn_detection.to_json().unwrap();
        let deserialized_turn_detection: TurnDetection = TurnDetection::from_json(&json).unwrap();

        assert_eq!(
            turn_detection.get_type(),
            deserialized_turn_detection.get_type()
        );
        assert_eq!(
            turn_detection.get_threshold(),
            deserialized_turn_detection.get_threshold()
        );
    }

    #[test]
    fn test_tool_serialization() {
        let tool = Tool {
            r#type: "function".to_string(),
            name: "get_weather".to_string(),
            description: "Get current weather".to_string(),
            parameters: ToolParameters {
                r#type: "object".to_string(),
                properties: Some(HashMap::from([(
                    "location".to_string(),
                    serde_json::json!({"type": "string"}),
                )])),
                required: Some(vec!["location".to_string()]),
            },
        };

        let json = tool.to_json().unwrap();
        let deserialized_tool: Tool = Tool::from_json(&json).unwrap();

        assert_eq!(tool.get_name(), deserialized_tool.get_name());
        assert_eq!(tool.get_type(), deserialized_tool.get_type());
    }
}
