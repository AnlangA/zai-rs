//! Content moderation API models and types.
//!
//! This module provides data structures for content moderation requests and
//! responses, supporting text, image, audio, and video content safety analysis.
//!
//! ## Features
//!
//! - **Multi-format support** - Text, image, audio, and video content
//!   moderation
//! - **Risk detection** - Identifies pornographic, violent, and illegal content
//! - **Structured results** - Detailed risk level and type information
//! - **Validation** - Input validation using the validator crate

use serde::{Deserialize, Deserializer, Serialize};
use validator::Validate;

// Helper: accept string or number and always deserialize into Option<String>
fn de_opt_string_from_number_or_string<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let v = serde_json::Value::deserialize(deserializer)?;
    match v {
        serde_json::Value::Null => Ok(None),
        serde_json::Value::String(s) => Ok(Some(s)),
        serde_json::Value::Number(n) => Ok(Some(n.to_string())),
        other => Err(serde::de::Error::custom(format!(
            "expected string or number, got {}",
            other
        ))),
    }
}

/// Content moderation model type.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum ModerationModel {
    /// Default moderation model
    #[serde(rename = "moderation")]
    #[default]
    Moderation,
}

/// Moderation input content.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ModerationInput {
    /// Text content for moderation
    Text(String),
    /// Multimedia content with type and URL
    Multimedia(MultimediaInput),
}

/// Multimedia input for content moderation.
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct MultimediaInput {
    /// Content type (image, audio, video)
    #[serde(rename = "type")]
    pub content_type: MediaType,
    /// URL to the multimedia content
    #[validate(url)]
    pub url: String,
}

/// Media types supported for moderation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MediaType {
    /// Image content
    #[serde(rename = "image")]
    Image,
    /// Audio content
    #[serde(rename = "audio")]
    Audio,
    /// Video content
    #[serde(rename = "video")]
    Video,
}

/// Content moderation request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModerationRequest {
    /// Moderation model to use
    #[serde(default)]
    pub model: ModerationModel,
    /// Content to moderate
    pub input: ModerationInput,
}

impl ModerationRequest {
    /// Create a new moderation request with text content.
    pub fn new_text(text: impl Into<String>) -> Self {
        Self {
            model: ModerationModel::default(),
            input: ModerationInput::Text(text.into()),
        }
    }

    /// Create a new moderation request with multimedia content.
    pub fn new_multimedia(content_type: MediaType, url: impl Into<String>) -> Self {
        Self {
            model: ModerationModel::default(),
            input: ModerationInput::Multimedia(MultimediaInput {
                content_type,
                url: url.into(),
            }),
        }
    }

    /// Validates the moderation request parameters.
    pub fn validate(&self) -> Result<(), validator::ValidationErrors> {
        let mut errors = validator::ValidationErrors::new();

        // Validate text input length
        if let ModerationInput::Text(text) = &self.input
            && text.len() > 2000
        {
            errors.add(
                "input",
                validator::ValidationError::new("text_length_exceeded"),
            );
        }

        // Validate multimedia URL
        if let ModerationInput::Multimedia(multimedia) = &self.input
            && multimedia.url.parse::<url::Url>().is_err()
        {
            errors.add("input", validator::ValidationError::new("invalid_url"));
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

/// Risk level for moderated content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RiskLevel {
    /// Normal content, no risks detected
    #[serde(rename = "PASS")]
    Pass,
    /// Suspicious content, requires review
    #[serde(rename = "REVIEW")]
    Review,
    /// Violating content, should be rejected
    #[serde(rename = "REJECT")]
    Reject,
}

/// Risk types that can be detected.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RiskType {
    /// Pornographic or adult content
    #[serde(rename = "porn")]
    Porn,
    /// Violent or gory content
    #[serde(rename = "violence")]
    Violence,
    /// Illegal or criminal content
    #[serde(rename = "illegal")]
    Illegal,
    /// Political or sensitive content
    #[serde(rename = "politics")]
    Politics,
    /// Other risk types
    #[serde(rename = "other")]
    Other,
}

/// Moderation result for a single content item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModerationResult {
    /// Type of content that was moderated
    #[serde(rename = "content_type")]
    pub content_type: String,
    /// Risk level assessment
    #[serde(rename = "risk_level")]
    pub risk_level: RiskLevel,
    /// List of detected risk types
    #[serde(rename = "risk_type")]
    pub risk_types: Vec<String>,
}

/// Usage statistics for moderation API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModerationUsage {
    /// Text moderation usage statistics
    #[serde(rename = "moderation_text")]
    pub moderation_text: ModerationTextUsage,
}

/// Text moderation usage statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModerationTextUsage {
    /// Number of text moderation calls
    #[serde(rename = "call_count")]
    pub call_count: u32,
}

/// Content moderation response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModerationResponse {
    /// Task ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Request creation time (Unix timestamp in seconds)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created: Option<u64>,
    /// Request identifier
    #[serde(
        rename = "request_id",
        skip_serializing_if = "Option::is_none",
        deserialize_with = "de_opt_string_from_number_or_string"
    )]
    pub request_id: Option<String>,
    /// List of moderation results
    #[serde(rename = "result_list", skip_serializing_if = "Option::is_none")]
    pub result_list: Option<Vec<ModerationResult>>,
    /// Usage statistics
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<ModerationUsage>,
}
