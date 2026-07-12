use serde::Serialize;
use validator::Validate;

use super::super::traits::*;
use crate::ZaiResult;
use crate::{ZaiError, client::error::codes};

/// Body parameters holder for audio transcription (used to build the multipart
/// form).
///
/// `prompt` is optional and the upstream 8,000-character guidance remains
/// advisory. The SDK validates the hot-word count and identifier lengths.
#[derive(Clone, Serialize, Validate)]
#[validate(schema(function = "validate_audio_to_text_body"))]
pub struct AudioToTextBody<N>
where
    N: AudioToText,
{
    /// Model code (e.g., `glm-asr-2512`).
    pub(super) model: N,

    /// Optional prompt (the 8,000-character guidance is advisory).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) prompt: Option<String>,

    /// Hot-word list, at most 100 entries.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[validate(length(max = 100))]
    pub(super) hotwords: Vec<String>,

    /// Whether the server must return an SSE response.
    pub(super) stream: bool,

    /// Client-provided unique request id (6..=64 chars).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(length(min = 6, max = 64))]
    pub(super) request_id: Option<String>,

    /// End-user id (6..=128 chars).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(length(min = 6, max = 128))]
    pub(super) user_id: Option<String>,
}

impl<N> std::fmt::Debug for AudioToTextBody<N>
where
    N: AudioToText + std::fmt::Debug,
{
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AudioToTextBody")
            .field("model", &self.model)
            .field("prompt_configured", &self.prompt.is_some())
            .field("hotword_count", &self.hotwords.len())
            .field("stream", &self.stream)
            .field("request_id_configured", &self.request_id.is_some())
            .field("user_id_configured", &self.user_id.is_some())
            .finish()
    }
}

fn validate_audio_to_text_body<N>(
    body: &AudioToTextBody<N>,
) -> Result<(), validator::ValidationError>
where
    N: AudioToText,
{
    if body
        .prompt
        .as_deref()
        .is_some_and(|prompt| prompt.trim().is_empty())
    {
        return Err(validator::ValidationError::new("prompt_must_not_be_blank"));
    }
    if body
        .hotwords
        .iter()
        .any(|hotword| hotword.trim().is_empty())
    {
        return Err(validator::ValidationError::new(
            "hotwords_must_not_be_blank",
        ));
    }
    if body.request_id.as_deref().is_some_and(|id| id.trim() != id)
        || body.user_id.as_deref().is_some_and(|id| id.trim() != id)
    {
        return Err(validator::ValidationError::new(
            "identifier_must_not_have_surrounding_whitespace",
        ));
    }
    Ok(())
}

impl<N> AudioToTextBody<N>
where
    N: AudioToText,
{
    /// Create a new transcription body for the given model (all options empty).
    pub fn new(model: N) -> Self {
        Self {
            model,
            prompt: None,
            hotwords: Vec::new(),
            stream: false,
            request_id: None,
            user_id: None,
        }
    }

    /// Borrow the selected model marker.
    pub fn model(&self) -> &N {
        &self.model
    }

    /// Borrow the optional transcription context prompt.
    pub fn prompt(&self) -> Option<&str> {
        self.prompt.as_deref()
    }

    /// Borrow the configured hot words.
    pub fn hotwords(&self) -> &[String] {
        &self.hotwords
    }

    /// Whether this body requests an SSE response.
    pub fn is_streaming(&self) -> bool {
        self.stream
    }

    /// Borrow the optional client request identifier.
    pub fn request_id(&self) -> Option<&str> {
        self.request_id.as_deref()
    }

    /// Borrow the optional end-user identifier.
    pub fn user_id(&self) -> Option<&str> {
        self.user_id.as_deref()
    }

    pub(super) fn with_stream(mut self, stream: bool) -> Self {
        self.stream = stream;
        self
    }

    /// Set an optional prompt (advisory 8000-char limit; not hard-rejected).
    pub fn with_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.prompt = Some(prompt.into());
        self
    }

    /// Set the hot-word list (at most 100 entries).
    pub fn with_hotwords(mut self, hotwords: Vec<String>) -> ZaiResult<Self> {
        if hotwords.len() > 100 {
            return Err(ZaiError::ApiError {
                code: codes::SDK_VALIDATION,
                message: "hotwords must contain at most 100 entries".to_string(),
            });
        }
        self.hotwords = hotwords;
        Ok(self)
    }

    /// Set the client-side request id (6..=64 chars).
    pub fn with_request_id(mut self, request_id: impl Into<String>) -> Self {
        self.request_id = Some(request_id.into());
        self
    }

    /// Set the end-user id (6..=128 chars).
    pub fn with_user_id(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = Some(user_id.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::audio_to_text::GlmAsr;

    #[test]
    fn validation_rejects_blank_optional_text_fields() {
        assert!(
            AudioToTextBody::new(GlmAsr {})
                .with_prompt(" ")
                .validate()
                .is_err()
        );
        assert!(
            AudioToTextBody::new(GlmAsr {})
                .with_hotwords(vec!["valid".into(), String::new()])
                .unwrap()
                .validate()
                .is_err()
        );
    }

    #[test]
    fn identifiers_and_hotwords_follow_frozen_limits() {
        assert!(
            AudioToTextBody::new(GlmAsr {})
                .with_request_id("short")
                .validate()
                .is_err()
        );
        assert!(
            AudioToTextBody::new(GlmAsr {})
                .with_request_id(" request-1 ")
                .validate()
                .is_err()
        );
        assert!(
            AudioToTextBody::new(GlmAsr {})
                .with_user_id("12345")
                .validate()
                .is_err()
        );
        assert!(
            AudioToTextBody::new(GlmAsr {})
                .with_request_id("r".repeat(64))
                .with_user_id("u".repeat(128))
                .validate()
                .is_ok()
        );
        assert!(
            AudioToTextBody::new(GlmAsr {})
                .with_prompt("p".repeat(8_001))
                .validate()
                .is_ok(),
            "the 8,000-character prompt guidance is advisory"
        );
    }

    #[test]
    fn debug_redacts_request_content_and_identifiers() {
        let body = AudioToTextBody::new(GlmAsr {})
            .with_prompt("private transcript")
            .with_hotwords(vec!["private-hotword".into()])
            .unwrap()
            .with_request_id("request-secret")
            .with_user_id("user-secret");
        let debug = format!("{body:?}");
        for secret in [
            "private transcript",
            "private-hotword",
            "request-secret",
            "user-secret",
        ] {
            assert!(!debug.contains(secret));
        }
        assert!(debug.contains("hotword_count: 1"));
    }
}
