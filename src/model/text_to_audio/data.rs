use serde::Serialize;
use validator::Validate;

use super::{
    super::traits::*,
    request::{TextToAudioBody, TtsAudioFormat, Voice},
};
use crate::client::ZaiClient;

/// Text-to-speech request wrapper using a JSON body.
///
/// Builder for the text-to-speech endpoint. Construct with
/// [`TextToAudioRequest::new`], tune with the `with_*` methods, then call
/// [`TextToAudioRequest::send_via`].
///
/// The endpoint returns raw audio, so [`send_via`](Self::send_via) yields
/// [`bytes::Bytes`] rather than a JSON response type.
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
    /// audio bytes (the endpoint does not return JSON).
    pub async fn send_via(&self, client: &ZaiClient) -> crate::ZaiResult<bytes::Bytes> {
        self.validate()?;
        let route = crate::client::routes::AUDIO_SYNTHESIZE;
        let url = client.endpoints().resolve_route(route, &[])?;
        client
            .send_json_bytes(route.method(), url, &self.body)
            .await
    }
}
