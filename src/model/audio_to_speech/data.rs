use super::super::traits::*;
use super::request::{TtsAudioFormat, TtsRequestBody, TtsVoice};
use crate::client::http::HttpClient;
use serde::Serialize;

/// Text-to-speech request wrapper using JSON body
pub struct TtsSpeechRequest<N>
where
    N: ModelName + TextToSpeech + Serialize,
{
    pub key: String,
    body: TtsRequestBody<N>,
}

impl<N> TtsSpeechRequest<N>
where
    N: ModelName + TextToSpeech + Serialize,
{
    pub fn new(model: N, key: String) -> Self {
        let body = TtsRequestBody::new(model);
        Self { key, body }
    }

    pub fn body_mut(&mut self) -> &mut TtsRequestBody<N> {
        &mut self.body
    }

    pub fn with_input(mut self, input: impl Into<String>) -> Self {
        self.body = self.body.with_input(input);
        self
    }
    pub fn with_voice(mut self, voice: TtsVoice) -> Self {
        self.body = self.body.with_voice(voice);
        self
    }
    pub fn with_speed(mut self, speed: f32) -> Self {
        self.body = self.body.with_speed(speed);
        self
    }
    pub fn with_volume(mut self, volume: f32) -> Self {
        self.body = self.body.with_volume(volume);
        self
    }
    pub fn with_response_format(mut self, fmt: TtsAudioFormat) -> Self {
        self.body = self.body.with_response_format(fmt);
        self
    }
    pub fn with_watermark_enabled(mut self, enabled: bool) -> Self {
        self.body = self.body.with_watermark_enabled(enabled);
        self
    }
}

impl<N> HttpClient for TtsSpeechRequest<N>
where
    N: ModelName + TextToSpeech + Serialize,
{
    type Body = TtsRequestBody<N>;
    type ApiUrl = &'static str;
    type ApiKey = String;

    fn api_url(&self) -> &Self::ApiUrl {
        &"https://open.bigmodel.cn/api/paas/v4/audio/speech"
    }
    fn api_key(&self) -> &Self::ApiKey {
        &self.key
    }
    fn body(&self) -> &Self::Body {
        &self.body
    }
}
