use super::request::VoiceListQuery;
use crate::ZaiResult;
use crate::client::ZaiClient;

/// GET voice list request
///
/// Builder for the voice-list endpoint. Construct with
/// [`VoiceListRequest::new`], optionally refine with
/// [`VoiceListRequest::with_query`], then call [`VoiceListRequest::send_via`].
///
/// **P05**: credentials and transport live on the [`ZaiClient`], passed to
/// [`send_via`](Self::send_via).
pub struct VoiceListRequest {
    query: VoiceListQuery,
}

impl VoiceListRequest {
    /// Create a new voice-list request with default (empty) query.
    ///
    /// **P05**: no longer takes an API key — the key is provided by the
    /// [`ZaiClient`] at send time.
    pub fn new() -> Self {
        Self {
            query: VoiceListQuery::new(),
        }
    }

    /// Validate the request (no required params; always succeeds).
    pub fn validate(&self) -> ZaiResult<()> {
        // No required params. Optionally, validate query formats here.
        Ok(())
    }

    /// Submit the GET request via a [`ZaiClient`] and parse the typed
    /// voice-list response.
    pub async fn send_via(
        &self,
        client: &ZaiClient,
    ) -> ZaiResult<super::response::VoiceListResponse> {
        self.validate()?;
        // Resolve base + path + any optional query parameters.
        let mut params: Vec<(&str, String)> = Vec::new();
        if let Some(ref n) = self.query.voice_name {
            params.push(("voiceName", n.clone()));
        }
        if let Some(ref t) = self.query.voice_type {
            params.push(("voiceType", t.as_str().to_string()));
        }
        let owned: Vec<(String, String)> = params
            .iter()
            .map(|(k, v)| ((*k).to_string(), v.clone()))
            .collect();
        let query_refs: Vec<(&str, &str)> = owned
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();
        let route = crate::client::routes::AUDIO_LIST_VOICES;
        let url = client
            .endpoints()
            .resolve_route_with_query(route, &[], &query_refs)?;
        client
            .send_empty::<super::response::VoiceListResponse>(route.method(), url)
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
