use serde::Serialize;
use validator::Validate;

use super::super::traits::*;

/// Request body for voice cloning.
#[derive(Clone, Serialize, Validate)]
#[validate(schema(function = "validate_voice_clone_body"))]
pub struct VoiceCloneBody<N>
where
    N: VoiceClone,
{
    /// Model code, e.g. `glm-tts-clone`.
    pub model: N,

    /// Unique voice name to register
    #[validate(length(min = 1, max = 128))]
    pub voice_name: String,

    /// Target text to synthesize for preview
    #[validate(length(min = 1, max = 4096))]
    pub input: String,

    /// File id of the example audio uploaded via file API
    #[validate(length(min = 1))]
    pub file_id: String,

    /// Optional: transcription text of the example audio
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,

    /// Optional: client-provided request id
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(length(min = 6, max = 64))]
    pub request_id: Option<String>,
}

impl<N> std::fmt::Debug for VoiceCloneBody<N>
where
    N: VoiceClone + std::fmt::Debug,
{
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("VoiceCloneBody")
            .field("model", &self.model)
            .field("voice_name", &"[REDACTED]")
            .field("input", &"[REDACTED]")
            .field("file_id", &"[REDACTED]")
            .field("text_configured", &self.text.is_some())
            .field(
                "request_id",
                &self.request_id.as_ref().map(|_| "[REDACTED]"),
            )
            .finish()
    }
}

fn validate_voice_clone_body<N>(body: &VoiceCloneBody<N>) -> Result<(), validator::ValidationError>
where
    N: VoiceClone,
{
    if body.voice_name.trim().is_empty()
        || body.input.trim().is_empty()
        || body.file_id.trim().is_empty()
    {
        return Err(validator::ValidationError::new(
            "required_fields_must_not_be_blank",
        ));
    }
    if body
        .text
        .as_deref()
        .is_some_and(|text| text.trim().is_empty())
    {
        return Err(validator::ValidationError::new("text_must_not_be_blank"));
    }
    Ok(())
}

impl<N> VoiceCloneBody<N>
where
    N: VoiceClone,
{
    /// Create a new voice-clone body from the required fields.
    pub fn new(
        model: N,
        voice_name: impl Into<String>,
        input: impl Into<String>,
        file_id: impl Into<String>,
    ) -> Self {
        Self {
            model,
            voice_name: voice_name.into(),
            input: input.into(),
            file_id: file_id.into(),
            text: None,
            request_id: None,
        }
    }

    /// Set the optional transcription text of the example audio.
    pub fn with_text(mut self, text: impl Into<String>) -> Self {
        self.text = Some(text.into());
        self
    }

    /// Set the optional client-provided request id.
    pub fn with_request_id(mut self, request_id: impl Into<String>) -> Self {
        self.request_id = Some(request_id.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::voice_clone::GlmTtsClone;

    #[test]
    fn debug_redacts_all_caller_supplied_text_and_ids() {
        let body = VoiceCloneBody::new(
            GlmTtsClone {},
            "private-name",
            "private input",
            "private-file",
        )
        .with_text("private transcript")
        .with_request_id("private-request");
        let debug = format!("{body:?}");
        for secret in [
            "private-name",
            "private input",
            "private-file",
            "private transcript",
            "private-request",
        ] {
            assert!(!debug.contains(secret));
        }
    }
}
