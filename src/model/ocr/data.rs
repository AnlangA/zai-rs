use std::path::Path;

use validator::Validate;

use super::request::{OcrBody, OcrLanguageType, OcrToolType};
use crate::client::http::{HttpClient, HttpClientConfig, http_client_with_config};

/// OCR recognition request (multipart/form-data)
pub struct OcrRequest {
    pub key: String,
    pub body: OcrBody,
    file_path: Option<String>,
}

impl OcrRequest {
    pub fn new(key: String) -> Self {
        Self {
            key,
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
                code: 0,
                message: format!("file_path not found: {}", p),
            });
        }

        // Validate file size (max 8MB)
        let metadata = std::fs::metadata(p)?;
        let file_size = metadata.len();
        const MAX_SIZE: u64 = 8 * 1024 * 1024; // 8MB
        if file_size > MAX_SIZE {
            return Err(crate::client::error::ZaiError::FileError {
                code: 0,
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
                code: 0,
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

        let parsed = resp.json::<super::response::OcrResponse>().await?;

        Ok(parsed)
    }
}

impl HttpClient for OcrRequest {
    type Body = OcrBody;
    type ApiUrl = &'static str;
    type ApiKey = String;

    fn api_url(&self) -> &Self::ApiUrl {
        &"https://open.bigmodel.cn/api/paas/v4/files/ocr"
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
        let url = (*self.api_url()).to_string();
        let body = self.body.clone();
        let file_path_opt = self.file_path.clone();

        async move {
            let file_path =
                file_path_opt.ok_or_else(|| crate::client::error::ZaiError::ApiError {
                    code: 1200,
                    message: "file_path is required".to_string(),
                })?;

            let mut form = reqwest::multipart::Form::new();

            // file
            let file_name = Path::new(&file_path)
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("image.png");
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

            let part = reqwest::multipart::Part::bytes(file_bytes)
                .file_name(file_name.to_string())
                .mime_str(mime)?;
            form = form.part("file", part);

            // tool_type (required, default to hand_write)
            let tool_type_str = match &body.tool_type {
                Some(OcrToolType::HandWrite) => "hand_write",
                None => "hand_write",
            };
            form = form.text("tool_type", tool_type_str);

            // language_type (optional)
            if let Some(lang) = &body.language_type {
                let lang_str = serde_json::to_string(lang)
                    .unwrap_or_default()
                    .trim_matches('"')
                    .to_string();
                form = form.text("language_type", lang_str);
            }

            // probability (optional)
            if let Some(prob) = body.probability {
                form = form.text("probability", prob.to_string());
            }

            // Use shared HTTP client with connection pooling
            let client = http_client_with_config(&HttpClientConfig::default());
            let resp = client
                .post(url)
                .bearer_auth(key)
                .multipart(form)
                .send()
                .await?;

            Ok(resp)
        }
    }
}
