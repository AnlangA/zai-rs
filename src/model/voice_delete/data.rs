use validator::Validate;

use super::request::VoiceDeleteBody;
use crate::client::ZaiClient;

/// Voice-deletion request using a JSON body.
///
/// Builder for the voice-delete endpoint. Construct with
/// [`VoiceDeleteRequest::new`], tune with the `with_*` methods, then call
/// [`VoiceDeleteRequest::send_via`].
pub struct VoiceDeleteRequest {
    body: VoiceDeleteBody,
}

impl VoiceDeleteRequest {
    /// Create a new voice-delete request for the given voice id.
    pub fn new(voice: impl Into<String>) -> Self {
        let body = VoiceDeleteBody::new(voice);
        Self { body }
    }

    /// Set the client-side request id.
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

    /// Submit the request via a [`ZaiClient`] and parse the typed voice-delete
    /// response.
    pub async fn send_via(
        &self,
        client: &ZaiClient,
    ) -> crate::ZaiResult<super::response::VoiceDeleteResponse> {
        self.validate()?;
        let route = crate::client::routes::AUDIO_DELETE_VOICE;
        client
            .operation(route)
            .send_json::<_, super::response::VoiceDeleteResponse>(&self.body)
            .await
    }
}
