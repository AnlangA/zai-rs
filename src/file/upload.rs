use std::path::PathBuf;

use crate::client::http::HttpClient;
use super::request::FilePurpose;

/// File upload request (multipart/form-data)
///
/// Sends a multipart request with fields:
/// - purpose: `FilePurpose`
/// - file: file content
pub struct FileUploadRequest {
    pub key: String,
    purpose: FilePurpose,
    file_path: PathBuf,
    file_name: Option<String>,
    content_type: Option<String>,
}

impl FileUploadRequest {
    pub fn new(key: String, purpose: FilePurpose, file_path: impl Into<PathBuf>) -> Self {
        Self {
            key,
            purpose,
            file_path: file_path.into(),
            file_name: None,
            content_type: None,
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
}

impl HttpClient for FileUploadRequest {
    type Body = (); // unused
    type ApiUrl = &'static str;
    type ApiKey = String;

    fn api_url(&self) -> &Self::ApiUrl { &"https://open.bigmodel.cn/api/paas/v4/files" }
    fn api_key(&self) -> &Self::ApiKey { &self.key }
    fn body(&self) -> &Self::Body { &() }

    // Override POST to send multipart/form-data
    fn post(&self) -> impl std::future::Future<Output = anyhow::Result<reqwest::Response>> + Send {
        let url: String = "https://open.bigmodel.cn/api/paas/v4/files".to_string();
        let key: String = self.key.clone();
        let purpose = self.purpose.clone();
        let path = self.file_path.clone();
        let file_name = self.file_name.clone();
        let content_type = self.content_type.clone();
        async move {
            let mut form = reqwest::multipart::Form::new().text("purpose", purpose.as_str().to_string());

            let fname = file_name
                .or_else(|| path.file_name().and_then(|s| s.to_str()).map(|s| s.to_string()))
                .unwrap_or_else(|| "upload.bin".to_string());

            let mut part = reqwest::multipart::Part::bytes(std::fs::read(&path)?).file_name(fname);
            if let Some(ct) = content_type {
                part = part.mime_str(&ct).map_err(|e| anyhow::anyhow!("invalid content-type: {}", e))?;
            }
            form = form.part("file", part);

            let resp = reqwest::Client::new()
                .post(url)
                .bearer_auth(key)
                .multipart(form)
                .send()
                .await?;

            let status = resp.status();
            if status.is_success() {
                return Ok(resp);
            }

            // Parse standard error envelope {"error": { code, message }}
            let text = resp.text().await.unwrap_or_default();
            #[derive(serde::Deserialize)]
            struct ErrEnv { error: ErrObj }
            #[derive(serde::Deserialize)]
            struct ErrObj { code: serde_json::Value, message: String }
            if let Ok(parsed) = serde_json::from_str::<ErrEnv>(&text) {
                return Err(anyhow::anyhow!(
                    "HTTP {} {} | code={} | message={}",
                    status.as_u16(),
                    status.canonical_reason().unwrap_or(""),
                    parsed.error.code,
                    parsed.error.message
                ));
            } else {
                return Err(anyhow::anyhow!(
                    "HTTP {} {} | body={}",
                    status.as_u16(),
                    status.canonical_reason().unwrap_or(""),
                    text
                ));
            }
        }
    }
}

