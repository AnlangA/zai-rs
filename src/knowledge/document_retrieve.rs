use std::sync::Arc;

use super::types::DocumentDetailResponse;
use crate::{
    ZaiResult,
    client::{
        endpoints::{ApiBase, EndpointConfig, join_url, paths},
        http::{HttpClient, HttpClientConfig, parse_typed_response},
    },
};

/// Retrieve document detail by id
pub struct DocumentRetrieveRequest {
    /// Bearer API key
    pub key: String,
    url: String,
    endpoint_config: EndpointConfig,
    api_base: ApiBase,
    document_id: String,
    http_config: Arc<HttpClientConfig>,
    _body: (),
}

impl DocumentRetrieveRequest {
    /// Create a new request
    pub fn new(key: String, document_id: impl AsRef<str>) -> Self {
        let document_id = document_id.as_ref().to_string();
        let endpoint_config = EndpointConfig::default();
        let api_base = ApiBase::LlmApplication;
        let url = endpoint_config.url(&api_base, &join_url(paths::DOCUMENT, &document_id));
        Self {
            key,
            url,
            endpoint_config,
            api_base,
            document_id,
            http_config: Arc::new(HttpClientConfig::default()),
            _body: (),
        }
    }

    fn rebuild_url(&mut self) {
        self.url = self.endpoint_config.url(
            &self.api_base,
            &join_url(paths::DOCUMENT, &self.document_id),
        );
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

    /// Send GET request and parse typed response
    pub async fn send(&self) -> ZaiResult<DocumentDetailResponse> {
        let resp = self.get().await?;
        let parsed = parse_typed_response::<DocumentDetailResponse>(resp).await?;
        Ok(parsed)
    }
}

impl HttpClient for DocumentRetrieveRequest {
    type Body = (); // unused
    type ApiUrl = String;
    type ApiKey = String;

    fn api_url(&self) -> &Self::ApiUrl {
        &self.url
    }
    fn api_key(&self) -> &Self::ApiKey {
        &self.key
    }
    fn body(&self) -> &Self::Body {
        &self._body
    }

    fn http_config(&self) -> Arc<HttpClientConfig> {
        Arc::clone(&self.http_config)
    }
}
