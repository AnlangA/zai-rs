use validator::Validate;

use super::{super::traits::*, request::VoiceCloneBody};
use crate::client::ZaiClient;

/// Voice-cloning request wrapper using a JSON body.
///
/// Builder for the voice-clone endpoint. Construct with
/// [`VoiceCloneRequest::new`], tune with the `with_*` methods, then call
/// [`VoiceCloneRequest::send_via`].
pub struct VoiceCloneRequest<N>
where
    N: VoiceClone,
{
    body: VoiceCloneBody<N>,
}

impl<N> VoiceCloneRequest<N>
where
    N: VoiceClone,
{
    /// Create a new voice clone request with required fields.
    pub fn new(
        model: N,
        voice_name: impl Into<String>,
        input: impl Into<String>,
        file_id: impl Into<String>,
    ) -> Self {
        let body = VoiceCloneBody::new(model, voice_name, input, file_id);
        Self { body }
    }

    /// Borrow the underlying [`VoiceCloneBody`] mutably for advanced tweaks.
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

    /// Validate the request body constraints before sending.
    pub fn validate(&self) -> crate::ZaiResult<()> {
        self.body
            .validate()
            .map_err(crate::client::error::ZaiError::from)?;
        Ok(())
    }

    /// Submit the request via a [`ZaiClient`] and parse the typed voice-clone
    /// response.
    pub async fn send_via(
        &self,
        client: &ZaiClient,
    ) -> crate::ZaiResult<super::response::VoiceCloneResponse> {
        self.validate()?;
        let route = crate::client::routes::AUDIO_CLONE_VOICE;
        client
            .operation(route)
            .send_json::<_, super::response::VoiceCloneResponse>(&self.body)
            .await
    }
}
