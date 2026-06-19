use std::sync::Arc;

use super::types::DocumentImageListResponse;
use crate::client::{
    endpoints::{ApiBase, EndpointConfig, join_url, paths},
    http::{HttpClient, HttpClientConfig, parse_typed_response, send_empty_request},
};

/// Retrieve parsed image index-url mapping for a document (POST, no body)
pub struct DocumentImageListRequest {
    /// Bearer API key
    pub key: String,
    url: String,
    endpoint_config: EndpointConfig,
    api_base: ApiBase,
    document_id: String,
    http_config: Arc<HttpClientConfig>,
    _body: (),
}

impl DocumentImageListRequest {
    /// Create a new request with the target document id
    pub fn new(key: String, document_id: impl AsRef<str>) -> Self {
        let document_id = document_id.as_ref().to_string();
        let endpoint_config = EndpointConfig::default();
        let api_base = ApiBase::LlmApplication;
        let url = endpoint_config.url(
            &api_base,
            &join_url(paths::DOCUMENT_SLICE_IMAGE_LIST, &document_id),
        );
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
            &join_url(paths::DOCUMENT_SLICE_IMAGE_LIST, &self.document_id),
        );
    }

    /// Override the base URL (uses [`ApiBase::Custom`]).
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.api_base = ApiBase::Custom(base_url.into());
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

    /// Send POST request and parse typed response
    pub async fn send(&self) -> crate::ZaiResult<DocumentImageListResponse> {
        let resp: reqwest::Response = self.post().await?;
        parse_typed_response::<DocumentImageListResponse>(resp).await
    }
}

impl HttpClient for DocumentImageListRequest {
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

    // Override POST: send no body, only auth header
    fn post(
        &self,
    ) -> impl std::future::Future<Output = crate::ZaiResult<reqwest::Response>> + Send {
        let url = self.url.clone();
        let key = self.key.clone();
        let config = self.http_config.clone();
        async move { send_empty_request(reqwest::Method::POST, url, key, config).await }
    }

    fn http_config(&self) -> Arc<HttpClientConfig> {
        self.http_config.clone()
    }
}
