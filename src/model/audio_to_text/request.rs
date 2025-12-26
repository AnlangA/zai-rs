use super::super::traits::*;
use serde::Serialize;
use validator::Validate;

/// Body parameters holder for audio transcription (used to build multipart form)
#[derive(Debug, Clone, Serialize, Validate)]
pub struct AudioToTextBody<N>
where
    N: ModelName + AudioToText + Serialize,
{
    /// Model code (e.g., glm-asr)
    pub model: N,

    /// Sampling temperature [0.0, 1.0], default 0.95
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(range(min = 0.0, max = 1.0))]
    pub temperature: Option<f32>,

    /// Stream mode flag (sync call should keep false or omit)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,

    /// Client-provided unique request id
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,

    /// End user id (6..=128 chars)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(length(min = 6, max = 128))]
    pub user_id: Option<String>,
}

impl<N> AudioToTextBody<N>
where
    N: ModelName + AudioToText + Serialize,
{
    pub fn new(model: N) -> Self {
        Self {
            model,
            temperature: None,
            stream: None,
            request_id: None,
            user_id: None,
        }
    }

    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }

    pub fn with_stream(mut self, stream: bool) -> Self {
        self.stream = Some(stream);
        self
    }

    pub fn with_request_id(mut self, request_id: impl Into<String>) -> Self {
        self.request_id = Some(request_id.into());
        self
    }

    pub fn with_user_id(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = Some(user_id.into());
        self
    }
}
