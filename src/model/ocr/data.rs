use std::{path::Path, sync::Arc};

use validator::Validate;

use super::request::{OcrBody, OcrLanguageType, OcrToolType};
use crate::client::{
    endpoints::{ApiBase, EndpointConfig, paths},
    error::codes,
    http::{HttpClient, HttpClientConfig, parse_typed_response, send_multipart_request},
};

/// OCR recognition request (multipart/form-data)
pub struct OcrRequest {
    pub key: String,
    url: String,
    endpoint_config: EndpointConfig,
    api_base: ApiBase,
    http_config: Arc<HttpClientConfig>,
    pub body: OcrBody,
    file_path: Option<String>,
}

impl OcrRequest {
    pub fn new(key: String) -> Self {
        let endpoint_config = EndpointConfig::default();
        let api_base = ApiBase::PaasV4;
        let url = endpoint_config.url(&api_base, paths::FILES_OCR);
        Self {
            key,
            url,
            endpoint_config,
            api_base,
            http_config: Arc::new(HttpClientConfig::default()),
            body: OcrBody::new(),
            file_path: None,
        }
    }

    pub fn with_file_path(mut self, path: impl Into<String>) -> Self {
        self.file_path = Some(path.into());
        self
    }

    pub fn with_tool_type(mut self, tool_type: OcrToolType) -> Self {
        self.body = self.body.with_tool_type(tool_type);
        self
    }

    pub fn with_language_type(mut self, language_type: OcrLanguageType) -> Self {
        self.body = self.body.with_language_type(language_type);
        self
    }

    pub fn with_probability(mut self, probability: bool) -> Self {
        self.body = self.body.with_probability(probability);
        self
    }

    pub fn with_request_id(mut self, request_id: impl Into<String>) -> Self {
        self.body = self.body.with_request_id(request_id);
        self
    }

    pub fn with_user_id(mut self, user_id: impl Into<String>) -> Self {
        self.body = self.body.with_user_id(user_id);
        self
    }

    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.api_base = ApiBase::Custom(base_url.into());
        self.url = self.endpoint_config.url(&self.api_base, paths::FILES_OCR);
        self
    }

    pub fn with_endpoint_config(mut self, endpoint_config: EndpointConfig) -> Self {
        self.endpoint_config = endpoint_config;
        self.url = self.endpoint_config.url(&self.api_base, paths::FILES_OCR);
        self
    }

    pub fn with_http_config(mut self, config: HttpClientConfig) -> Self {
        self.http_config = Arc::new(config);
        self
    }

    pub fn validate(&self) -> crate::ZaiResult<()> {
        // Check body constraints
        self.body
            .validate()
            .map_err(crate::client::error::ZaiError::from)?;

        // Ensure file path exists
        let p =
            self.file_path
                .as_ref()
                .ok_or_else(|| crate::client::error::ZaiError::ApiError {
                    code: 1200,
                    message: "file_path is required".to_string(),
                })?;

        if !Path::new(p).exists() {
            return Err(crate::client::error::ZaiError::FileError {
                code: codes::SDK_FILE_NOT_FOUND,
                message: format!("file_path not found: {}", p),
            });
        }

        // Validate file size (max 8MB)
        let metadata = std::fs::metadata(p)?;
        let file_size = metadata.len();
        const MAX_SIZE: u64 = 8 * 1024 * 1024; // 8MB
        if file_size > MAX_SIZE {
            return Err(crate::client::error::ZaiError::FileError {
                code: codes::SDK_FILE_TOO_LARGE,
                message: format!("file_size exceeds 8MB limit: {} bytes", file_size),
            });
        }

        // Validate file extension
        let ext = Path::new(p)
            .extension()
            .and_then(|s| s.to_str())
            .map(|s| s.to_ascii_lowercase());
        let valid_ext = matches!(
            ext.as_deref(),
            Some("png") | Some("jpg") | Some("jpeg") | Some("bmp")
        );
        if !valid_ext {
            return Err(crate::client::error::ZaiError::FileError {
                code: codes::SDK_FILE_TYPE_UNSUPPORTED,
                message: format!(
                    "invalid file format: {:?}. Only PNG, JPG, JPEG, BMP are supported",
                    ext
                ),
            });
        }

        Ok(())
    }

    pub async fn send(&self) -> crate::ZaiResult<super::response::OcrResponse> {
        self.validate()?;

        let resp = self.post().await?;

        parse_typed_response::<super::response::OcrResponse>(resp).await
    }
}

impl HttpClient for OcrRequest {
    type Body = OcrBody;
    type ApiUrl = String;
    type ApiKey = String;

    fn api_url(&self) -> &Self::ApiUrl {
        &self.url
    }

    fn api_key(&self) -> &Self::ApiKey {
        &self.key
    }

    fn body(&self) -> &Self::Body {
        &self.body
    }

    fn post(
        &self,
    ) -> impl std::future::Future<Output = crate::ZaiResult<reqwest::Response>> + Send {
        let key = self.key.clone();
        let url = self.url.clone();
        let config = self.http_config.clone();
        let body = self.body.clone();
        let file_path_opt = self.file_path.clone();

        async move {
            let file_path =
                file_path_opt.ok_or_else(|| crate::client::error::ZaiError::ApiError {
                    code: 1200,
                    message: "file_path is required".to_string(),
                })?;

            let file_name = Path::new(&file_path)
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("image.png")
                .to_string();
            let file_bytes = tokio::fs::read(&file_path).await?;

            // Determine MIME type by extension
            let ext = Path::new(&file_path)
                .extension()
                .and_then(|s| s.to_str())
                .map(|s| s.to_ascii_lowercase());
            let mime = match ext.as_deref() {
                Some("png") => "image/png",
                Some("jpg") | Some("jpeg") => "image/jpeg",
                Some("bmp") => "image/bmp",
                _ => "image/png",
            };

            let tool_type_str = match &body.tool_type {
                Some(OcrToolType::HandWrite) => "hand_write",
                None => "hand_write",
            }
            .to_string();

            let language_type = body.language_type.as_ref().map(|lang| {
                serde_json::to_string(lang)
                    .unwrap_or_default()
                    .trim_matches('"')
                    .to_string()
            });
            let probability = body.probability;
            let request_id = body.request_id.clone();
            let user_id = body.user_id.clone();

            send_multipart_request(reqwest::Method::POST, url, key, config, move || {
                let part = reqwest::multipart::Part::bytes(file_bytes.clone())
                    .file_name(file_name.clone())
                    .mime_str(mime)?;
                let mut form = reqwest::multipart::Form::new()
                    .part("file", part)
                    .text("tool_type", tool_type_str.clone());
                if let Some(lang) = language_type.as_ref() {
                    form = form.text("language_type", lang.clone());
                }
                if let Some(prob) = probability {
                    form = form.text("probability", prob.to_string());
                }
                if let Some(rid) = request_id.as_ref() {
                    form = form.text("request_id", rid.clone());
                }
                if let Some(uid) = user_id.as_ref() {
                    form = form.text("user_id", uid.clone());
                }
                Ok(form)
            })
            .await
        }
    }

    fn http_config(&self) -> Arc<HttpClientConfig> {
        self.http_config.clone()
    }
}
