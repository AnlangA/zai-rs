use std::sync::Arc;

use serde::Serialize;

use super::{
    super::traits::*,
    request::{TextToAudioBody, TtsAudioFormat, Voice},
};
use crate::client::{
    endpoints::{ApiBase, EndpointConfig, paths},
    http::{HttpClient, HttpClientConfig},
};

/// Text-to-speech request wrapper using JSON body
///
/// Builder for the text-to-speech endpoint. Construct with
/// [`TextToAudioRequest::new`], tune with the `with_*` methods, then `post()`.
pub struct TextToAudioRequest<N>
where
    N: ModelName + TextToAudio + Serialize,
{
    /// Zhipu AI API key used for `Authorization: Bearer …`.
    pub key: String,
    url: String,
    endpoint_config: EndpointConfig,
    api_base: ApiBase,
    http_config: Arc<HttpClientConfig>,
    body: TextToAudioBody<N>,
}

impl<N> TextToAudioRequest<N>
where
    N: ModelName + TextToAudio + Serialize,
{
    /// Create a new TTS request for the given model.
    pub fn new(model: N, key: String) -> Self {
        let body = TextToAudioBody::new(model);
        let endpoint_config = EndpointConfig::default();
        let api_base = ApiBase::PaasV4;
        let url = endpoint_config.url(&api_base, paths::AUDIO_SPEECH);
        Self {
            key,
            url,
            endpoint_config,
            api_base,
            http_config: Arc::new(HttpClientConfig::default()),
            body,
        }
    }

    fn rebuild_url(&mut self) {
        self.url = self
            .endpoint_config
            .url(&self.api_base, paths::AUDIO_SPEECH);
    }

    /// Override the base URL (uses [`ApiBase::Custom`]).
    pub fn with_base_url(mut self, base: impl Into<String>) -> Self {
        self.api_base = ApiBase::Custom(base.into());
        self.rebuild_url();
        self
    }

    /// Replace the full [`EndpointConfig`] used to resolve URLs.
    pub fn with_endpoint_config(mut self, endpoint_config: EndpointConfig) -> Self {
        self.endpoint_config = endpoint_config;
        self.rebuild_url();
        self
    }

    /// Replace the HTTP client configuration (timeouts, retries, …).
    pub fn with_http_config(mut self, config: HttpClientConfig) -> Self {
        self.http_config = Arc::new(config);
        self
    }

    /// Borrow the underlying [`TextToAudioBody`] mutably for advanced tweaks.
    pub fn body_mut(&mut self) -> &mut TextToAudioBody<N> {
        &mut self.body
    }

    /// Set the input text to synthesize.
    pub fn with_input(mut self, input: impl Into<String>) -> Self {
        self.body = self.body.with_input(input);
        self
    }
    /// Set the voice preset.
    pub fn with_voice(mut self, voice: Voice) -> Self {
        self.body = self.body.with_voice(voice);
        self
    }
    /// Set the playback speed.
    pub fn with_speed(mut self, speed: f32) -> Self {
        self.body = self.body.with_speed(speed);
        self
    }
    /// Set the playback volume.
    pub fn with_volume(mut self, volume: f32) -> Self {
        self.body = self.body.with_volume(volume);
        self
    }
    /// Set the output audio format.
    pub fn with_response_format(mut self, fmt: TtsAudioFormat) -> Self {
        self.body = self.body.with_response_format(fmt);
        self
    }
    /// Enable/disable the audio watermark.
    pub fn with_watermark_enabled(mut self, enabled: bool) -> Self {
        self.body = self.body.with_watermark_enabled(enabled);
        self
    }
}

impl<N> HttpClient for TextToAudioRequest<N>
where
    N: ModelName + TextToAudio + Serialize,
{
    type Body = TextToAudioBody<N>;
    type ApiUrl = String;
    type ApiKey = String;

    /// Resolved target URL for the request.
    fn api_url(&self) -> &Self::ApiUrl {
        &self.url
    }
    /// API key used for `Authorization: Bearer …`.
    fn api_key(&self) -> &Self::ApiKey {
        &self.key
    }
    /// Serialized request body.
    fn body(&self) -> &Self::Body {
        &self.body
    }

    /// HTTP client configuration (timeouts, retries, …).
    fn http_config(&self) -> Arc<HttpClientConfig> {
        Arc::clone(&self.http_config)
    }
}
