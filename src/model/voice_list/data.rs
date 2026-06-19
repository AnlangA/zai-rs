use std::sync::Arc;

use super::request::VoiceListQuery;
use crate::{
    ZaiResult,
    client::{
        endpoints::{ApiBase, EndpointConfig, build_query, paths},
        http::{HttpClient, HttpClientConfig, parse_typed_response},
    },
};

/// GET voice list request
///
/// Builder for the voice-list endpoint. Construct with
/// [`VoiceListRequest::new`], optionally refine with
/// [`VoiceListRequest::with_query`], then call [`VoiceListRequest::send`].
pub struct VoiceListRequest {
    /// Zhipu AI API key used for `Authorization: Bearer …`.
    pub key: String,
    url: String,
    endpoint_config: EndpointConfig,
    api_base: ApiBase,
    http_config: Arc<HttpClientConfig>,
    query: VoiceListQuery,
    // Empty body placeholder to satisfy HttpClient::Body
    _body: (),
}

impl VoiceListRequest {
    /// Create a new voice-list request with default (empty) query.
    pub fn new(key: String) -> Self {
        let endpoint_config = EndpointConfig::default();
        let api_base = ApiBase::PaasV4;
        let url = endpoint_config.url(&api_base, paths::VOICE_LIST);
        Self {
            key,
            url,
            endpoint_config,
            api_base,
            http_config: Arc::new(HttpClientConfig::default()),
            query: VoiceListQuery::new(),
            _body: (),
        }
    }

    fn rebuild_url(&mut self) {
        let endpoint = self.endpoint_config.url(&self.api_base, paths::VOICE_LIST);
        let mut params: Vec<(&str, String)> = Vec::new();
        if let Some(ref n) = self.query.voice_name {
            params.push(("voiceName", n.clone()));
        }
        if let Some(ref t) = self.query.voice_type {
            params.push(("voiceType", t.as_str().to_string()));
        }
        self.url = build_query(&endpoint, params);
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

    /// Validate the request (no required params; always succeeds).
    pub fn validate(&self) -> ZaiResult<()> {
        // No required params; URL already built. Optionally, validate query formats
        // here.
        Ok(())
    }

    /// Submit the GET request and parse the typed voice-list response.
    pub async fn send(&self) -> ZaiResult<super::response::VoiceListResponse> {
        self.validate()?;
        let resp = self.get().await?;
        let parsed = parse_typed_response::<super::response::VoiceListResponse>(resp).await?;
        Ok(parsed)
    }

    /// Replace the query parameters (voice name / type filter).
    pub fn with_query(mut self, q: VoiceListQuery) -> Self {
        self.query = q;
        self.rebuild_url();
        self
    }
}

impl HttpClient for VoiceListRequest {
    type Body = ();
    type ApiUrl = String;
    type ApiKey = String;

    /// Resolved target URL (with query string) for the request.
    fn api_url(&self) -> &Self::ApiUrl {
        &self.url
    }
    /// API key used for `Authorization: Bearer …`.
    fn api_key(&self) -> &Self::ApiKey {
        &self.key
    }
    /// Empty body placeholder (GET request).
    fn body(&self) -> &Self::Body {
        &self._body
    }
    /// HTTP client configuration (timeouts, retries, …).
    fn http_config(&self) -> Arc<HttpClientConfig> {
        Arc::clone(&self.http_config)
    }
}
