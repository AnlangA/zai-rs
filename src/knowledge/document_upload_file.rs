use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::client::http::HttpClient;
use validator::Validate;

use super::types::UploadFileResponse;

/// Slice type (knowledge_type)
#[derive(Debug, Clone, Copy)]
pub enum DocumentSliceType {
    /// 1: Title-paragraph slicing (txt, doc, pdf, url, docx, ppt, pptx, md)
    TitleParagraph = 1,
    /// 2: Q&A slicing (txt, doc, pdf, url, docx, ppt, pptx, md)
    QaPair = 2,
    /// 3: Line slicing (xls, xlsx, csv)
    Line = 3,
    /// 5: Custom slicing (txt, doc, pdf, url, docx, ppt, pptx, md)
    Custom = 5,
    /// 6: Page slicing (pdf, ppt, pptx)
    Page = 6,
    /// 7: Single slice (xls, xlsx, csv)
    Single = 7,
}
impl DocumentSliceType {
    fn as_i64(self) -> i64 {
        self as i64
    }
}

/// Optional parameters for file upload
#[derive(Debug, Clone, Default, Validate)]
pub struct UploadFileOptions {
    /// Document type; if omitted, the server parses dynamically
    pub knowledge_type: Option<DocumentSliceType>,
    /// Custom slicing rules; used when knowledge_type = 5
    pub custom_separator: Option<Vec<String>>,
    /// Custom slice size; used when knowledge_type = 5; valid range: 20..=2000
    #[validate(range(min = 20, max = 2000))]
    pub sentence_size: Option<u32>,
    /// Whether to parse images
    pub parse_image: Option<bool>,
    /// Callback URL
    #[validate(url)]
    pub callback_url: Option<String>,
    /// Callback headers
    pub callback_header: Option<BTreeMap<String, String>>,
    /// Document word number limit (must be numeric string per API)
    pub word_num_limit: Option<String>,
    /// Request id
    #[validate(length(min = 1))]
    pub req_id: Option<String>,
}

/// File upload request (multipart/form-data)
pub struct DocumentUploadFileRequest {
    /// Bearer API key
    pub key: String,
    url: String,
    files: Vec<PathBuf>,
    options: UploadFileOptions,
}

impl DocumentUploadFileRequest {
    /// Create a new request for a specific knowledge base id
    pub fn new(key: String, knowledge_id: impl AsRef<str>) -> Self {
        let url = format!(
            "https://open.bigmodel.cn/api/llm-application/open/document/upload_document/{}",
            knowledge_id.as_ref()
        );
        Self {
            key,
            url,
            files: Vec::new(),
            options: UploadFileOptions::default(),
        }
    }

    /// Add a local file path to upload
    pub fn add_file_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.files.push(path.into());
        self
    }

    /// Set optional parameters
    pub fn with_options(mut self, opts: UploadFileOptions) -> Self {
        self.options = opts;
        self
    }

    /// Mutable access to options for incremental configuration
    pub fn options_mut(&mut self) -> &mut UploadFileOptions {
        &mut self.options
    }

    /// Validate cross-field constraints not expressible via `validator`
    fn validate_cross(&self) -> crate::ZaiResult<()> {
        // When knowledge_type is Custom (5), sentence_size should be within 20..=2000
        if let Some(DocumentSliceType::Custom) = self.options.knowledge_type {
            // sentence_size recommended; API shows default 300; we ensure range if provided
            if let Some(sz) = self.options.sentence_size
                && !(20..=2000).contains(&sz) {
                    return Err(crate::client::error::ZaiError::ApiError {
                        code: 1200,
                        message: "sentence_size must be 20..=2000 when knowledge_type=Custom (5)"
                            .to_string(),
                    });
                }
        }
        if let Some(ref w) = self.options.word_num_limit
            && !w.chars().all(|c| c.is_ascii_digit()) {
                return Err(crate::client::error::ZaiError::ApiError {
                    code: 1200,
                    message: "word_num_limit must be numeric string".to_string(),
                });
            }
        if self.files.is_empty() {
            return Err(crate::client::error::ZaiError::ApiError {
                code: 1200,
                message: "at least one file path must be provided".to_string(),
            });
        }
        Ok(())
    }

    /// Send multipart request and parse typed response
    pub async fn send(&self) -> crate::ZaiResult<UploadFileResponse> {
        // Field validations
        self.options.validate()?;
        self.validate_cross()?;

        let resp = self.post().await?;
        let parsed = resp.json::<UploadFileResponse>().await?;
        Ok(parsed)
    }
}

impl HttpClient for DocumentUploadFileRequest {
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
        &()
    }

    // Override POST to send multipart/form-data

    fn post(
        &self,
    ) -> impl std::future::Future<Output = crate::ZaiResult<reqwest::Response>> + Send {
        let url = self.url.clone();
        let key = self.key.clone();
        let files = self.files.clone();
        let opts = self.options.clone();
        async move {
            let mut form = reqwest::multipart::Form::new();

            // Optional fields
            if let Some(t) = opts.knowledge_type {
                form = form.text("knowledge_type", t.as_i64().to_string());
            }
            if let Some(seps) = opts.custom_separator.as_ref() {
                let s = serde_json::to_string(seps).unwrap_or("[]".to_string());
                form = form.text("custom_separator", s);
            }
            if let Some(sz) = opts.sentence_size {
                form = form.text("sentence_size", sz.to_string());
            }
            if let Some(pi) = opts.parse_image {
                form = form.text("parse_image", if pi { "true" } else { "false" }.to_string());
            }
            if let Some(u) = opts.callback_url.as_ref() {
                form = form.text("callback_url", u.clone());
            }
            if let Some(h) = opts.callback_header.as_ref() {
                let s = serde_json::to_string(h).unwrap_or("{}".to_string());
                form = form.text("callback_header", s);
            }
            if let Some(w) = opts.word_num_limit.as_ref() {
                form = form.text("word_num_limit", w.clone());
            }
            if let Some(r) = opts.req_id.as_ref() {
                form = form.text("req_id", r.clone());
            }

            // Files: use field name "files" per API
            for path in files {
                let fname = path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "upload.bin".to_string());
                let part = reqwest::multipart::Part::bytes(std::fs::read(&path)?).file_name(fname);
                form = form.part("files", part);
            }

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

            // Standard error envelope {"error": { code, message }}
            let text = resp.text().await.unwrap_or_default();
            #[derive(serde::Deserialize)]
            struct ErrEnv {
                error: ErrObj,
            }
            #[derive(serde::Deserialize)]
            struct ErrObj {
                _code: serde_json::Value,
                message: String,
            }

            if let Ok(parsed) = serde_json::from_str::<ErrEnv>(&text) {
                Err(crate::client::error::ZaiError::from_api_response(
                    status.as_u16(),
                    0,
                    parsed.error.message,
                ))
            } else {
                Err(crate::client::error::ZaiError::from_api_response(
                    status.as_u16(),
                    0,
                    text,
                ))
            }
        }
    }
}
