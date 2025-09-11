use serde::Serialize;
use validator::Validate;

#[derive(Debug, Clone, Serialize, Validate)]
pub struct VoiceDeleteBody {
    /// 要删除的音色标识
    #[validate(length(min = 1))]
    pub voice: String,

    /// 可选：请求ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
}

impl VoiceDeleteBody {
    pub fn new(voice: impl Into<String>) -> Self {
        Self { voice: voice.into(), request_id: None }
    }
    pub fn with_request_id(mut self, request_id: impl Into<String>) -> Self {
        self.request_id = Some(request_id.into());
        self
    }
}

