use super::super::traits::*;
use super::request::VoiceCloneBody;
use crate::client::http::HttpClient;
use serde::Serialize;
use validator::Validate;

/// Voice clone request wrapper using JSON
pub struct VoiceCloneRequest<N>
where
    N: ModelName + VoiceClone + Serialize,
{
    pub key: String,
    body: VoiceCloneBody<N>,
}

impl<N> VoiceCloneRequest<N>
where
    N: ModelName + VoiceClone + Serialize,
{
    /// Create a new voice clone request with required fields
    pub fn new(
        model: N,
        key: String,
        voice_name: impl Into<String>,
        input: impl Into<String>,
        file_id: impl Into<String>,
    ) -> Self {
        let body = VoiceCloneBody::new(model, voice_name, input, file_id);
        Self { key, body }
    }

    pub fn body_mut(&mut self) -> &mut VoiceCloneBody<N> {
        &mut self.body
    }

    /// Optional: reference text of the example audio
    pub fn with_text(mut self, text: impl Into<String>) -> Self {
        self.body = self.body.with_text(text);
        self
    }

    /// Optional: client-provided request id
    pub fn with_request_id(mut self, request_id: impl Into<String>) -> Self {
        self.body = self.body.with_request_id(request_id);
        self
    }

    pub fn validate(&self) -> anyhow::Result<()> {
        self.body.validate().map_err(|e| anyhow::anyhow!(e))?;
        Ok(())
    }

    pub async fn send(&self) -> anyhow::Result<super::response::VoiceCloneResponse> {
        self.validate()?;
        let resp = self.post().await?;
        let parsed = resp.json::<super::response::VoiceCloneResponse>().await?;
        Ok(parsed)
    }
}

impl<N> HttpClient for VoiceCloneRequest<N>
where
    N: ModelName + VoiceClone + Serialize,
{
    type Body = VoiceCloneBody<N>;
    type ApiUrl = &'static str;
    type ApiKey = String;

    fn api_url(&self) -> &Self::ApiUrl {
        &"https://open.bigmodel.cn/api/paas/v4/voice/clone"
    }
    fn api_key(&self) -> &Self::ApiKey {
        &self.key
    }
    fn body(&self) -> &Self::Body {
        &self.body
    }
}
