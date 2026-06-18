use std::{path::PathBuf, sync::Arc};

use super::request::FilePurpose;
use crate::client::{
    endpoints::{ApiBase, EndpointConfig, paths},
    http::{HttpClient, HttpClientConfig, parse_typed_response, send_multipart_request},
};

/// File upload request (multipart/form-data)
///
/// Sends a multipart request with fields:
/// - purpose: `FilePurpose`
/// - file: file content
pub struct FileUploadRequest {
    pub key: String,
    url: String,
    endpoint_config: EndpointConfig,
    api_base: ApiBase,
    http_config: Arc<HttpClientConfig>,
    purpose: FilePurpose,
    file_path: PathBuf,
    file_name: Option<String>,
    content_type: Option<String>,
    _body: (),
}

impl FileUploadRequest {
    pub fn new(key: String, purpose: FilePurpose, file_path: impl Into<PathBuf>) -> Self {
        let endpoint_config = EndpointConfig::default();
        let api_base = ApiBase::PaasV4;
        let url = endpoint_config.url(&api_base, paths::FILES);
        Self {
            key,
            url,
            endpoint_config,
            api_base,
            http_config: Arc::new(HttpClientConfig::default()),
            purpose,
            file_path: file_path.into(),
            file_name: None,
            content_type: None,
            _body: (),
        }
    }

    pub fn with_file_name(mut self, name: impl Into<String>) -> Self {
        self.file_name = Some(name.into());
        self
    }

    pub fn with_content_type(mut self, ct: impl Into<String>) -> Self {
        self.content_type = Some(ct.into());

        self
    }

    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.api_base = ApiBase::Custom(base_url.into());
        self.url = self.endpoint_config.url(&self.api_base, paths::FILES);
        self
    }

    pub fn with_endpoint_config(mut self, endpoint_config: EndpointConfig) -> Self {
        self.endpoint_config = endpoint_config;
        self.url = self.endpoint_config.url(&self.api_base, paths::FILES);
        self
    }

    pub fn with_http_config(mut self, config: HttpClientConfig) -> Self {
        self.http_config = Arc::new(config);
        self
    }

    /// Send the upload request and parse typed response (`FileObject`)
    pub async fn send(&self) -> crate::ZaiResult<super::response::FileObject> {
        let resp: reqwest::Response = self.post().await?;
        parse_typed_response::<super::response::FileObject>(resp).await
    }
}

impl HttpClient for FileUploadRequest {
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

    // Override POST to send multipart/form-data

    fn post(
        &self,
    ) -> impl std::future::Future<Output = crate::ZaiResult<reqwest::Response>> + Send {
        let url = self.url.clone();
        let key: String = self.key.clone();
        let config = self.http_config.clone();
        let purpose = self.purpose.clone();
        let path = self.file_path.clone();
        let file_name = self.file_name.clone();
        let content_type = self.content_type.clone();
        async move {
            let fname = file_name
                .or_else(|| {
                    path.file_name()
                        .and_then(|s| s.to_str())
                        .map(|s| s.to_string())
                })
                .unwrap_or_else(|| "upload.bin".to_string());

            let bytes = std::fs::read(&path)?;
            send_multipart_request(reqwest::Method::POST, url, key, config, move || {
                let mut part =
                    reqwest::multipart::Part::bytes(bytes.clone()).file_name(fname.clone());
                if let Some(ct) = content_type.as_ref() {
                    part = part.mime_str(ct).map_err(|e| {
                        crate::client::error::ZaiError::ApiError {
                            code: 1200,
                            message: format!("invalid content-type: {}", e),
                        }
                    })?;
                }
                Ok(reqwest::multipart::Form::new()
                    .text("purpose", purpose.as_str().to_string())
                    .part("file", part))
            })
            .await
        }
    }

    fn http_config(&self) -> Arc<HttpClientConfig> {
        self.http_config.clone()
    }
}
