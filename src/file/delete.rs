use std::sync::Arc;

use crate::client::{
    endpoints::{ApiBase, EndpointConfig, join_url, paths},
    http::{HttpClient, HttpClientConfig, parse_typed_response},
};

/// File delete request (DELETE /paas/v4/files/{file_id})
pub struct FileDeleteRequest {
    /// Zhipu AI API key used for `Authorization: Bearer …`.
    pub key: String,
    url: String,
    endpoint_config: EndpointConfig,
    api_base: ApiBase,
    file_id: String,
    http_config: Arc<HttpClientConfig>,
    _body: (),
}

impl FileDeleteRequest {
    /// Create a new delete request for the given file id.
    pub fn new(key: String, file_id: impl Into<String>) -> Self {
        let file_id = file_id.into();
        let endpoint_config = EndpointConfig::default();
        let api_base = ApiBase::PaasV4;
        let url = endpoint_config.url(&api_base, &join_url(paths::FILES, &file_id));
        Self {
            key,
            url,
            endpoint_config,
            api_base,
            file_id,
            http_config: Arc::new(HttpClientConfig::default()),
            _body: (),
        }
    }

    fn rebuild_url(&mut self) {
        self.url = self
            .endpoint_config
            .url(&self.api_base, &join_url(paths::FILES, &self.file_id));
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

    /// Issue the underlying DELETE request.
    pub fn delete(
        &self,
    ) -> impl std::future::Future<Output = crate::ZaiResult<reqwest::Response>> + Send {
        HttpClient::delete(self)
    }

    /// Send delete request and parse typed response.
    pub async fn send(&self) -> crate::ZaiResult<super::response::FileDeleteResponse> {
        let resp = self.delete().await?;
        parse_typed_response::<super::response::FileDeleteResponse>(resp).await
    }
}

impl HttpClient for FileDeleteRequest {
    type Body = ();
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
    /// Empty body placeholder (DELETE request).
    fn body(&self) -> &Self::Body {
        &self._body
    }

    /// HTTP client configuration (timeouts, retries, …).
    fn http_config(&self) -> Arc<HttpClientConfig> {
        self.http_config.clone()
    }
}
