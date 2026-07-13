use super::request::VoiceListQuery;
use crate::ZaiResult;
use crate::client::ZaiClient;
use validator::Validate;

/// Builder for a `GET` request to list available voices.
///
/// Builder for the voice-list endpoint. Construct with
/// [`VoiceListRequest::new`], optionally refine with
/// [`VoiceListRequest::with_query`], then call [`VoiceListRequest::send_via`].
pub struct VoiceListRequest {
    query: VoiceListQuery,
}

impl VoiceListRequest {
    /// Create a new voice-list request with default (empty) query.
    pub fn new() -> Self {
        Self {
            query: VoiceListQuery::new(),
        }
    }

    /// Validate optional query fields before sending.
    pub fn validate(&self) -> ZaiResult<()> {
        self.query
            .validate()
            .map_err(crate::client::error::ZaiError::from)?;
        Ok(())
    }

    /// Submit the GET request via a [`ZaiClient`] and parse the typed
    /// voice-list response.
    pub async fn send_via(
        &self,
        client: &ZaiClient,
    ) -> ZaiResult<super::response::VoiceListResponse> {
        self.validate()?;
        let mut query = Vec::with_capacity(2);
        if let Some(name) = self.query.voice_name.as_deref() {
            query.push(("voiceName", name));
        }
        if let Some(voice_type) = self.query.voice_type.as_ref() {
            query.push(("voiceType", voice_type.as_str()));
        }
        let route = crate::client::routes::AUDIO_LIST_VOICES;
        client
            .operation(route)
            .with_query(query)
            .send_empty::<super::response::VoiceListResponse>()
            .await
    }

    /// Replace the query parameters (voice name / type filter).
    pub fn with_query(mut self, q: VoiceListQuery) -> Self {
        self.query = q;
        self
    }
}

impl Default for VoiceListRequest {
    fn default() -> Self {
        Self::new()
    }
}
