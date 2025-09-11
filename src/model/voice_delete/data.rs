use super::request::VoiceDeleteBody;
use crate::client::http::HttpClient;

/// Voice delete request using JSON body
pub struct VoiceDeleteRequest {
    pub key: String,
    body: VoiceDeleteBody,
}

impl VoiceDeleteRequest {
    pub fn new(key: String, voice: impl Into<String>) -> Self {
        let body = VoiceDeleteBody::new(voice);
        Self { key, body }
    }

    pub fn with_request_id(mut self, request_id: impl Into<String>) -> Self {
        self.body = self.body.with_request_id(request_id);
        self
    }
}

impl HttpClient for VoiceDeleteRequest {
    type Body = VoiceDeleteBody;
    type ApiUrl = &'static str;
    type ApiKey = String;

    fn api_url(&self) -> &Self::ApiUrl {
        &"https://open.bigmodel.cn/api/paas/v4/voice/delete"
    }
    fn api_key(&self) -> &Self::ApiKey {
        &self.key
    }
    fn body(&self) -> &Self::Body {
        &self.body
    }
}
