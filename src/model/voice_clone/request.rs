use super::super::traits::*;
use serde::Serialize;
use validator::Validate;

#[derive(Debug, Clone, Serialize, Validate)]
pub struct VoiceCloneBody<N>
where
    N: ModelName + VoiceClone + Serialize,
{
    /// Model code, e.g. "cogtts-clone"
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
    pub request_id: Option<String>,
}

impl<N> VoiceCloneBody<N>
where
    N: ModelName + VoiceClone + Serialize,
{
    pub fn new(model: N, voice_name: impl Into<String>, input: impl Into<String>, file_id: impl Into<String>) -> Self {
        Self {
            model,
            voice_name: voice_name.into(),
            input: input.into(),
            file_id: file_id.into(),
            text: None,
            request_id: None,
        }
    }

    pub fn with_text(mut self, text: impl Into<String>) -> Self {
        self.text = Some(text.into());
        self
    }

    pub fn with_request_id(mut self, request_id: impl Into<String>) -> Self {
        self.request_id = Some(request_id.into());
        self
    }
}

