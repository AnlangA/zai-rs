use std::{collections::BTreeMap, path::PathBuf};

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use validator::Validate;

use super::types::DocumentUploadResponse;
use crate::client::ZaiClient;

/// Slice type (knowledge_type)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i64)]
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
    /// Return the exact integer value used by the API.
    pub const fn as_i64(self) -> i64 {
        self as i64
    }
}

impl Serialize for DocumentSliceType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_i64(self.as_i64())
    }
}

impl<'de> Deserialize<'de> for DocumentSliceType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        match i64::deserialize(deserializer)? {
            1 => Ok(Self::TitleParagraph),
            2 => Ok(Self::QaPair),
            3 => Ok(Self::Line),
            5 => Ok(Self::Custom),
            6 => Ok(Self::Page),
            7 => Ok(Self::Single),
            value => Err(serde::de::Error::custom(format!(
                "unsupported document slice type: {value}"
            ))),
        }
    }
}

/// Optional parameters for file upload
#[derive(Clone, Default, Validate)]
pub struct DocumentUploadOptions {
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

impl std::fmt::Debug for DocumentUploadOptions {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("DocumentUploadOptions")
            .field("knowledge_type", &self.knowledge_type)
            .field(
                "custom_separator_count",
                &self.custom_separator.as_ref().map(Vec::len),
            )
            .field("sentence_size", &self.sentence_size)
            .field("parse_image", &self.parse_image)
            .field("callback_url_configured", &self.callback_url.is_some())
            .field(
                "callback_header_entries",
                &self.callback_header.as_ref().map(BTreeMap::len),
            )
            .field("word_num_limit_configured", &self.word_num_limit.is_some())
            .field("req_id", &self.req_id.as_ref().map(|_| "[REDACTED]"))
            .finish()
    }
}

/// File upload request (multipart/form-data)
///
/// Credentials and transport live on the [`ZaiClient`], passed to
/// [`send_via`](Self::send_via).
pub struct DocumentUploadRequest {
    knowledge_id: String,
    files: Vec<PathBuf>,
    options: DocumentUploadOptions,
}

impl DocumentUploadRequest {
    /// Create a new request for a specific knowledge base id
    pub fn new(knowledge_id: impl Into<String>) -> Self {
        Self {
            knowledge_id: knowledge_id.into(),
            files: Vec::new(),
            options: DocumentUploadOptions::default(),
        }
    }

    /// Add a local file path to upload
    pub fn add_file_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.files.push(path.into());
        self
    }

    /// Set optional parameters
    pub fn with_options(mut self, opts: DocumentUploadOptions) -> Self {
        self.options = opts;
        self
    }

    /// Validate cross-field constraints not expressible via `validator`
    fn validate_cross(&self) -> crate::ZaiResult<()> {
        crate::client::validation::require_non_blank(&self.knowledge_id, "knowledge_id")?;
        if let Some(ref w) = self.options.word_num_limit
            && (w.is_empty() || !w.chars().all(|c| c.is_ascii_digit()))
        {
            return Err(crate::client::validation::invalid(
                "word_num_limit must be numeric string",
            ));
        }
        if self.files.is_empty() {
            return Err(crate::client::validation::invalid(
                "at least one file path must be provided",
            ));
        }
        if self
            .options
            .custom_separator
            .as_ref()
            .is_some_and(|separators| {
                separators.is_empty() || separators.iter().any(|value| value.trim().is_empty())
            })
        {
            return Err(crate::client::validation::invalid(
                "custom_separator must contain at least one non-blank separator",
            ));
        }
        if (self.options.custom_separator.is_some() || self.options.sentence_size.is_some())
            && self.options.knowledge_type != Some(DocumentSliceType::Custom)
        {
            return Err(crate::client::validation::invalid(
                "custom_separator and sentence_size require knowledge_type=Custom",
            ));
        }
        if self
            .options
            .req_id
            .as_deref()
            .is_some_and(|request_id| request_id.trim().is_empty())
        {
            return Err(crate::client::validation::invalid(
                "req_id must not be blank",
            ));
        }
        Ok(())
    }

    /// Send the multipart request via a [`ZaiClient`] and parse the typed response.
    pub async fn send_via(&self, client: &ZaiClient) -> crate::ZaiResult<DocumentUploadResponse> {
        self.options.validate()?;
        self.validate_cross()?;

        let route = crate::client::routes::DOCUMENTS_UPLOAD;
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
            factory = factory.field("callback_url", url.as_str())?;
        }
        if let Some(header) = self.options.callback_header.as_ref() {
            factory = factory.field("callback_header", serde_json::to_string(header)?)?;
        }
        if let Some(limit) = self.options.word_num_limit.as_ref() {
            factory = factory.field("word_num_limit", limit.as_str())?;
        }
        if let Some(req_id) = self.options.req_id.as_ref() {
            factory = factory.field("req_id", req_id.as_str())?;
        }
        for path in &self.files {
            let part = crate::client::transport::multipart::FilePart::from_path_async(path).await?;
            factory = factory.file_named("files", part)?;
        }
        client
            .operation(route)
            .with_parameters([self.knowledge_id.as_str()])
            .send_multipart::<DocumentUploadResponse>(&factory)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn options_debug_redacts_callbacks_separators_and_request_id() {
        let options = DocumentUploadOptions {
            knowledge_type: Some(DocumentSliceType::Custom),
            custom_separator: Some(vec!["private-separator".to_owned()]),
            callback_url: Some("https://private.example/callback".to_owned()),
            callback_header: Some(BTreeMap::from([(
                "Authorization".to_owned(),
                "private-token".to_owned(),
            )])),
            word_num_limit: Some("private-limit".to_owned()),
            req_id: Some("private-request".to_owned()),
            ..DocumentUploadOptions::default()
        };
        let debug = format!("{options:?}");
        for secret in [
            "private-separator",
            "private.example",
            "Authorization",
            "private-token",
            "private-limit",
            "private-request",
        ] {
            assert!(!debug.contains(secret));
        }
        assert!(debug.contains("callback_header_entries: Some(1)"));
    }

    #[test]
    fn whitespace_only_custom_separator_is_rejected() {
        let request = DocumentUploadRequest::new("knowledge-1")
            .add_file_path("document.txt")
            .with_options(DocumentUploadOptions {
                knowledge_type: Some(DocumentSliceType::Custom),
                custom_separator: Some(vec!["  ".to_owned()]),
                ..DocumentUploadOptions::default()
            });
        assert!(request.validate_cross().is_err());
    }
}
