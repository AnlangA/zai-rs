use serde::Serialize;
use validator::Validate;

use super::super::traits::*;
use crate::ZaiResult;
use crate::{ZaiError, client::error::codes};

/// Body parameters holder for audio transcription (used to build the multipart
/// form).
///
/// `prompt` is optional and its upstream 8,000-character limit is left to the
/// server. The SDK validates the hot-word count and identifier lengths.
#[derive(Debug, Clone, Serialize, Validate)]
pub struct AudioToTextBody<N>
where
    N: ModelName + AudioToText + Serialize,
{
    /// Model code (e.g., `glm-asr-2512`).
    pub model: N,

    /// Optional prompt (advisory 8000-char limit; the server enforces it).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,

    /// Hot-word list, at most 100 entries.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[validate(length(max = 100))]
    pub hotwords: Vec<String>,

    /// Stream-mode flag.
    ///
    /// [`AudioToTextRequest::send_via`](super::data::AudioToTextRequest::send_via)
    /// decodes a single JSON response, so leave this unset or `false` when using
    /// that method.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,

    /// Client-provided unique request id (6..=64 chars).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(length(min = 6, max = 64))]
    pub request_id: Option<String>,

    /// End-user id (6..=128 chars).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(length(min = 6, max = 128))]
    pub user_id: Option<String>,
}

impl<N> AudioToTextBody<N>
where
    N: ModelName + AudioToText + Serialize,
{
    /// Create a new transcription body for the given model (all options empty).
    pub fn new(model: N) -> Self {
        Self {
            model,
            prompt: None,
            hotwords: Vec::new(),
            stream: None,
            request_id: None,
            user_id: None,
        }
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

    /// Enable/disable streaming responses.
    pub fn with_stream(mut self, stream: bool) -> Self {
        self.stream = Some(stream);
        self
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
