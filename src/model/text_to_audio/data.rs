use std::sync::Arc;

use serde::Serialize;
use validator::Validate;

use super::{
    super::traits::*,
    request::{TextToAudioBody, TtsAudioFormat, Voice},
};
use crate::client::ZaiClient;
use crate::client::http::{HttpClientConfig, send_json_request};

/// Text-to-speech request wrapper using JSON body
///
/// Builder for the text-to-speech endpoint. Construct with
/// [`TextToAudioRequest::new`], tune with the `with_*` methods, then call
/// [`TextToAudioRequest::send_via`].
///
/// **P05**: credentials and transport live on the [`ZaiClient`], passed to
/// [`send_via`](Self::send_via). The endpoint returns raw audio bytes, so
/// `send_via` yields the underlying `reqwest::Response` for the caller to
/// extract bytes from.
pub struct TextToAudioRequest<N>
where
    N: ModelName + TextToAudio + Serialize,
{
    body: TextToAudioBody<N>,
}

impl<N> TextToAudioRequest<N>
where
    N: ModelName + TextToAudio + Serialize,
{
    /// Create a new TTS request for the given model.
    ///
    /// **P05**: no longer takes an API key — the key is provided by the
    /// [`ZaiClient`] at send time.
    pub fn new(model: N) -> Self {
        let body = TextToAudioBody::new(model);
        Self { body }
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

    /// Validate the request body constraints before sending.
    pub fn validate(&self) -> crate::ZaiResult<()> {
        self.body
            .validate()
            .map_err(|e| crate::client::error::ZaiError::ApiError {
                code: crate::client::error::codes::SDK_VALIDATION,
                message: format!("Validation error: {e:?}"),
            })?;
        Ok(())
    }

    /// Submit the request via a [`ZaiClient`] and return the raw
    /// `reqwest::Response` (the endpoint yields audio bytes, not JSON).
    pub async fn send_via(&self, client: &ZaiClient) -> crate::ZaiResult<reqwest::Response> {
        self.validate()?;
        let url = client
            .endpoints()
            .resolve(crate::client::ApiFamily::PaasV4, &["audio", "speech"])?;
        let config = transport_config_from_client(client);
        let resp = send_json_request(
            reqwest::Method::POST,
            url,
            client.secret().expose(),
            &self.body,
            Arc::new(config),
        )
        .await?;
        Ok(resp)
    }
}

fn transport_config_from_client(client: &ZaiClient) -> HttpClientConfig {
    let t = client.transport();
    HttpClientConfig {
        timeout: std::time::Duration::from_secs(t.request_timeout.as_secs()),
        max_retries: u32::from(t.max_attempts).saturating_sub(1),
        enable_compression: t.enable_compression,
        retry_delay: crate::client::http::RetryDelay::Exponential {
            base: std::time::Duration::from_millis(500),
            max: std::time::Duration::from_secs(5),
        },
        enable_logging: false,
        mask_sensitive_data: true,
    }
}
