use std::{collections::BTreeMap, path::PathBuf};

use validator::Validate;

use super::types::UploadFileResponse;
use crate::client::ZaiClient;

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
///
/// Credentials and transport live on the [`ZaiClient`], passed to
/// [`send_via`](Self::send_via).
pub struct DocumentUploadFileRequest {
    knowledge_id: String,
    files: Vec<PathBuf>,
    options: UploadFileOptions,
}

impl DocumentUploadFileRequest {
    /// Create a new request for a specific knowledge base id
    pub fn new(knowledge_id: impl Into<String>) -> Self {
        Self {
            knowledge_id: knowledge_id.into(),
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
                && !(20..=2000).contains(&sz)
            {
                return Err(crate::client::error::ZaiError::ApiError {
                    code: crate::client::error::codes::SDK_VALIDATION,
                    message: "sentence_size must be 20..=2000 when knowledge_type=Custom (5)"
                        .to_string(),
                });
            }
        }
        if let Some(ref w) = self.options.word_num_limit
            && !w.chars().all(|c| c.is_ascii_digit())
        {
            return Err(crate::client::error::ZaiError::ApiError {
                code: crate::client::error::codes::SDK_VALIDATION,
                message: "word_num_limit must be numeric string".to_string(),
            });
        }
        if self.files.is_empty() {
            return Err(crate::client::error::ZaiError::ApiError {
                code: crate::client::error::codes::SDK_VALIDATION,
                message: "at least one file path must be provided".to_string(),
            });
        }
        Ok(())
    }

    /// Send the multipart request via a [`ZaiClient`] and parse the typed response.
    pub async fn send_via(&self, client: &ZaiClient) -> crate::ZaiResult<UploadFileResponse> {
        self.options.validate()?;
        self.validate_cross()?;

        let route = crate::client::routes::DOCUMENTS_UPLOAD;
        let url = client
            .endpoints()
            .resolve_route(route, &[&self.knowledge_id])?;
        let mut factory = crate::client::transport::multipart::MultipartBodyFactory::new();
        if let Some(t) = self.options.knowledge_type {
            factory = factory.field("knowledge_type", t.as_i64().to_string())?;
        }
        if let Some(seps) = self.options.custom_separator.as_ref() {
            factory = factory.field("custom_separator", serde_json::to_string(seps)?)?;
        }
        if let Some(sz) = self.options.sentence_size {
            factory = factory.field("sentence_size", sz.to_string())?;
        }
        if let Some(pi) = self.options.parse_image {
            factory = factory.field("parse_image", pi.to_string())?;
        }
        if let Some(url) = self.options.callback_url.as_ref() {
            factory = factory.field("callback_url", url.clone())?;
        }
        if let Some(header) = self.options.callback_header.as_ref() {
            factory = factory.field("callback_header", serde_json::to_string(header)?)?;
        }
        if let Some(limit) = self.options.word_num_limit.as_ref() {
            factory = factory.field("word_num_limit", limit.clone())?;
        }
        if let Some(req_id) = self.options.req_id.as_ref() {
            factory = factory.field("req_id", req_id.clone())?;
        }
        for path in &self.files {
            let part = crate::client::transport::multipart::FilePart::from_path(path)?;
            factory = factory.file_named("files", part)?;
        }
        client
            .send_multipart::<UploadFileResponse>(route.method(), url, &factory)
            .await
    }
}
