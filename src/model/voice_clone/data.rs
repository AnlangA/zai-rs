use std::sync::Arc;

use serde::Serialize;
use validator::Validate;

use super::{super::traits::*, request::VoiceCloneBody};
use crate::client::{
    endpoints::{ApiBase, EndpointConfig, paths},
    http::{HttpClient, HttpClientConfig, parse_typed_response},
};

/// Voice clone request wrapper using JSON
///
/// Builder for the voice-clone endpoint. Construct with
/// [`VoiceCloneRequest::new`], tune with the `with_*` methods, then call
/// [`VoiceCloneRequest::send`].
pub struct VoiceCloneRequest<N>
where
    N: ModelName + VoiceClone + Serialize,
{
    /// Zhipu AI API key used for `Authorization: Bearer …`.
    pub key: String,
    url: String,
    endpoint_config: EndpointConfig,
    api_base: ApiBase,
    http_config: Arc<HttpClientConfig>,
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
        let endpoint_config = EndpointConfig::default();
        let api_base = ApiBase::PaasV4;
        let url = endpoint_config.url(&api_base, paths::VOICE_CLONE);
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
        self.url = self.endpoint_config.url(&self.api_base, paths::VOICE_CLONE);
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
            .map_err(|e| crate::client::error::ZaiError::ApiError {
                code: 1200,
                message: format!("Validation error: {:?}", e),
            })?;
        Ok(())
    }

    /// Submit the request and parse the typed voice-clone response.
    pub async fn send(&self) -> crate::ZaiResult<super::response::VoiceCloneResponse> {
        self.validate()?;
        let resp = self.post().await?;
        let parsed = parse_typed_response::<super::response::VoiceCloneResponse>(resp).await?;
        Ok(parsed)
    }
}

impl<N> HttpClient for VoiceCloneRequest<N>
where
    N: ModelName + VoiceClone + Serialize,
{
    type Body = VoiceCloneBody<N>;
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
