use std::sync::Arc;

use validator::Validate;

use super::request::VoiceDeleteBody;
use crate::client::{
    endpoints::{ApiBase, EndpointConfig, paths},
    http::{HttpClient, HttpClientConfig, parse_typed_response},
};

/// Voice delete request using JSON body
///
/// Builder for the voice-delete endpoint. Construct with
/// [`VoiceDeleteRequest::new`], tune with the `with_*` methods, then call
/// [`VoiceDeleteRequest::send`].
pub struct VoiceDeleteRequest {
    /// Zhipu AI API key used for `Authorization: Bearer …`.
    pub key: String,
    url: String,
    endpoint_config: EndpointConfig,
    api_base: ApiBase,
    http_config: Arc<HttpClientConfig>,
    body: VoiceDeleteBody,
}

impl VoiceDeleteRequest {
    /// Create a new voice-delete request for the given voice id.
    pub fn new(key: impl Into<String>, voice: impl Into<String>) -> Self {
        let body = VoiceDeleteBody::new(voice);
        let endpoint_config = EndpointConfig::default();
        let api_base = ApiBase::PaasV4;
        let url = endpoint_config.url(&api_base, paths::VOICE_DELETE);
        Self {
            key: key.into(),
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
            .url(&self.api_base, paths::VOICE_DELETE);
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

    /// Set the client-side request id.
    pub fn with_request_id(mut self, request_id: impl Into<String>) -> Self {
        self.body = self.body.with_request_id(request_id);
        self
    }

    /// Validate the request body constraints before sending.
    pub fn validate(&self) -> crate::ZaiResult<()> {
        self.body
            .validate()
            .map_err(|e| crate::client::error::ZaiError::ApiError {
                code: crate::client::error::codes::SDK_VALIDATION,
                message: format!("Validation error: {:?}", e),
            })?;
        Ok(())
    }

    /// Submit the request and parse the typed voice-delete response.
    pub async fn send(&self) -> crate::ZaiResult<super::response::VoiceDeleteResponse> {
        self.validate()?;
        let resp = self.post().await?;
        let parsed = parse_typed_response::<super::response::VoiceDeleteResponse>(resp).await?;
        Ok(parsed)
    }
}

impl HttpClient for VoiceDeleteRequest {
    type Body = VoiceDeleteBody;
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
