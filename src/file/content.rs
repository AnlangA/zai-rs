use std::sync::Arc;

use crate::client::{
    endpoints::{ApiBase, EndpointConfig, join_url, paths},
    http::{HttpClient, HttpClientConfig},
};

/// File content request (GET /paas/v4/files/{file_id}/content)
pub struct FileContentRequest {
    /// Zhipu AI API key used for `Authorization: Bearer …`.
    pub key: String,
    url: String,
    endpoint_config: EndpointConfig,
    api_base: ApiBase,
    file_id: String,
    http_config: Arc<HttpClientConfig>,
    _body: (),
}

impl FileContentRequest {
    /// Create a new content request for the given file id.
    pub fn new(key: String, file_id: impl Into<String>) -> Self {
        let file_id = file_id.into();
        let endpoint_config = EndpointConfig::default();
        let api_base = ApiBase::PaasV4;
        let url = endpoint_config.url(
            &api_base,
            &join_url(&join_url(paths::FILES, &file_id), "content"),
        );
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
        self.url = self.endpoint_config.url(
            &self.api_base,
            &join_url(&join_url(paths::FILES, &self.file_id), "content"),
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

    /// Send the request and return raw bytes of the file content.
    pub async fn send(&self) -> crate::ZaiResult<Vec<u8>> {
        let resp: reqwest::Response = self.get().await?;

        let status = resp.status();

        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();

            return Err(crate::client::error::ZaiError::from_api_response(
                status.as_u16(),
                0,
                text,
            ));
        }

        let bytes = resp.bytes().await?;
        Ok(bytes.to_vec())
    }

    /// It will create parent directories if missing.
    /// Returns the number of bytes written.
    pub async fn send_to<P: AsRef<std::path::Path>>(&self, path: P) -> crate::ZaiResult<usize> {
        let bytes = self.send().await?;

        let p = path.as_ref();

        if let Some(parent) = p.parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(parent)?;
        }
        use std::io::Write;
        let mut f = std::fs::File::create(p)?;
        f.write_all(&bytes)?;
        Ok(bytes.len())
    }
}

impl HttpClient for FileContentRequest {
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
    /// Empty body placeholder (GET request).
    fn body(&self) -> &Self::Body {
        &self._body
    }

    /// HTTP client configuration (timeouts, retries, …).
    fn http_config(&self) -> Arc<HttpClientConfig> {
        self.http_config.clone()
    }
}
