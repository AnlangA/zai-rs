use serde::{Deserialize, Serialize};
use validator::Validate;

use super::{DocumentSliceType, types::DocumentUrlUploadResponse};
use crate::ZaiResult;
use crate::client::ZaiClient;

/// Single URL upload detail
#[derive(Clone, Serialize, Deserialize, Validate)]
pub struct DocumentUrlUploadDetail {
    /// Source URL to fetch
    #[validate(url)]
    pub url: String,
    /// Slice type (`1`, `2`, `3`, `5`, `6`, or `7`).
    pub knowledge_type: DocumentSliceType,
    /// Custom separators
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_separator: Option<Vec<String>>,
    /// Custom slice size (`20..=2000`).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(range(min = 20, max = 2000))]
    pub sentence_size: Option<u32>,
    /// Callback URL
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(url)]
    pub callback_url: Option<String>,
    /// Callback headers
    #[serde(skip_serializing_if = "Option::is_none")]
    pub callback_header: Option<std::collections::BTreeMap<String, String>>,
}

impl std::fmt::Debug for DocumentUrlUploadDetail {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("DocumentUrlUploadDetail")
            .field("url", &"[REDACTED]")
            .field("knowledge_type", &self.knowledge_type)
            .field(
                "custom_separator_count",
                &self.custom_separator.as_ref().map(Vec::len),
            )
            .field("sentence_size", &self.sentence_size)
            .field("callback_url_configured", &self.callback_url.is_some())
            .field(
                "callback_header_entries",
                &self.callback_header.as_ref().map(|headers| headers.len()),
            )
            .finish()
    }
}

impl DocumentUrlUploadDetail {
    /// Create a new upload detail for the given source URL.
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            // The upstream schema requires a slice type; title/paragraph
            // slicing is the broadest default for document URLs.
            knowledge_type: DocumentSliceType::TitleParagraph,
            custom_separator: None,
            sentence_size: None,
            callback_url: None,
            callback_header: None,
        }
    }
    /// Set the slice/knowledge type.
    pub fn with_knowledge_type(mut self, value: DocumentSliceType) -> Self {
        self.knowledge_type = value;
        self
    }
    /// Set custom separators.
    pub fn with_custom_separator(mut self, seps: Vec<String>) -> Self {
        self.custom_separator = Some(seps);
        self
    }
    /// Set the sentence size.
    pub fn with_sentence_size(mut self, size: u32) -> Self {
        self.sentence_size = Some(size);
        self
    }
    /// Set the callback URL.
    pub fn with_callback_url(mut self, url: impl Into<String>) -> Self {
        self.callback_url = Some(url.into());
        self
    }
    /// Set the callback headers.
    pub fn with_callback_header(
        mut self,
        headers: std::collections::BTreeMap<String, String>,
    ) -> Self {
        self.callback_header = Some(headers);
        self
    }

    fn validate_knowledge_type(&self) -> ZaiResult<()> {
        if self.custom_separator.as_ref().is_some_and(|values| {
            values.is_empty() || values.iter().any(|value| value.trim().is_empty())
        }) {
            return Err(crate::client::validation::invalid(
                "custom_separator must contain at least one non-blank value",
            ));
        }
        if (self.custom_separator.is_some() || self.sentence_size.is_some())
            && self.knowledge_type != DocumentSliceType::Custom
        {
            return Err(crate::client::validation::invalid(
                "custom_separator and sentence_size require knowledge_type=5",
            ));
        }
        Ok(())
    }
}

/// Upload URL request body
#[derive(Clone, Serialize, Deserialize, Validate)]
pub struct DocumentUrlUploadBody {
    /// Upload detail list (at least 1)
    #[validate(length(min = 1))]
    #[validate(nested)]
    pub upload_detail: Vec<DocumentUrlUploadDetail>,
    /// Knowledge base id
    #[validate(length(min = 1))]
    pub knowledge_id: String,
}

impl std::fmt::Debug for DocumentUrlUploadBody {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("DocumentUrlUploadBody")
            .field("upload_detail_count", &self.upload_detail.len())
            .field("knowledge_id", &"[REDACTED]")
            .finish()
    }
}

impl DocumentUrlUploadBody {
    /// Create a new upload body for the given knowledge base id.
    pub fn new(knowledge_id: impl Into<String>) -> Self {
        Self {
            upload_detail: Vec::new(),
            knowledge_id: knowledge_id.into(),
        }
    }
    /// Append a fully-built [`DocumentUrlUploadDetail`].
    pub fn add_detail(mut self, detail: DocumentUrlUploadDetail) -> Self {
        self.upload_detail.push(detail);
        self
    }
    /// Convenience: append a detail built from a single URL.
    pub fn add_url(mut self, url: impl Into<String>) -> Self {
        self.upload_detail.push(DocumentUrlUploadDetail::new(url));
        self
    }
}

/// Upload URL request (POST /llm-application/open/document/upload_url)
///
/// Credentials and transport live on the [`ZaiClient`], passed to
/// [`send_via`](Self::send_via).
pub struct DocumentUrlUploadRequest {
    body: DocumentUrlUploadBody,
}

impl DocumentUrlUploadRequest {
    /// Create a new upload-by-URL request with the given body.
    pub fn new(body: DocumentUrlUploadBody) -> Self {
        Self { body }
    }

    /// Validate identifiers, URLs, slice modes, and cross-field constraints.
    pub fn validate(&self) -> ZaiResult<()> {
        self.body.validate()?;
        crate::client::validation::require_non_blank(&self.body.knowledge_id, "knowledge_id")?;
        for detail in &self.body.upload_detail {
            detail.validate_knowledge_type()?;
        }
        Ok(())
    }

    /// Validate and send via a [`ZaiClient`], returning the typed response.
    pub async fn send_via(&self, client: &ZaiClient) -> ZaiResult<DocumentUrlUploadResponse> {
        self.validate()?;
        let route = crate::client::routes::DOCUMENTS_UPLOAD_URL;
        let url = client.endpoints().resolve_route(route, &[])?;
        client
            .send_json::<_, DocumentUrlUploadResponse>(route.method(), url, &self.body)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_required_ids_and_custom_slice_fields() {
        let blank_id = DocumentUrlUploadRequest::new(
            DocumentUrlUploadBody::new(" ").add_url("https://example.com/doc"),
        );
        assert!(blank_id.validate().is_err());

        let wrong_mode =
            DocumentUrlUploadRequest::new(DocumentUrlUploadBody::new("kb-1").add_detail(
                DocumentUrlUploadDetail::new("https://example.com/doc").with_sentence_size(300),
            ));
        assert!(wrong_mode.validate().is_err());

        let blank_separator = DocumentUrlUploadRequest::new(
            DocumentUrlUploadBody::new("kb-1").add_detail(
                DocumentUrlUploadDetail::new("https://example.com/doc")
                    .with_knowledge_type(DocumentSliceType::Custom)
                    .with_custom_separator(vec![" ".to_owned()]),
            ),
        );
        assert!(blank_separator.validate().is_err());

        let valid = DocumentUrlUploadRequest::new(
            DocumentUrlUploadBody::new("kb-1").add_detail(
                DocumentUrlUploadDetail::new("https://example.com/doc")
                    .with_knowledge_type(DocumentSliceType::Custom)
                    .with_sentence_size(300),
            ),
        );
        assert!(valid.validate().is_ok());
    }

    #[test]
    fn every_url_detail_serializes_the_required_typed_slice_mode() {
        let body = DocumentUrlUploadBody::new("kb-1").add_url("https://example.com/doc");
        let value = serde_json::to_value(body).unwrap();
        assert_eq!(value["upload_detail"][0]["knowledge_type"], 1);
        assert!(
            serde_json::from_value::<DocumentUrlUploadDetail>(serde_json::json!({
                "url": "https://example.com/doc"
            }))
            .is_err()
        );
        assert!(
            serde_json::from_value::<DocumentUrlUploadDetail>(serde_json::json!({
                "url": "https://example.com/doc",
                "knowledge_type": 4
            }))
            .is_err()
        );
    }

    #[test]
    fn request_debug_redacts_urls_ids_separators_and_headers() {
        let detail = DocumentUrlUploadDetail::new("https://private.example/document")
            .with_knowledge_type(DocumentSliceType::Custom)
            .with_custom_separator(vec!["private-separator".to_owned()])
            .with_callback_url("https://private.example/callback")
            .with_callback_header(std::collections::BTreeMap::from([(
                "Authorization".to_owned(),
                "private-token".to_owned(),
            )]));
        let detail_debug = format!("{detail:?}");
        for secret in [
            "private.example",
            "private-separator",
            "Authorization",
            "private-token",
        ] {
            assert!(!detail_debug.contains(secret));
        }

        let body = DocumentUrlUploadBody::new("private-knowledge").add_detail(detail);
        let body_debug = format!("{body:?}");
        assert!(!body_debug.contains("private-knowledge"));
        assert!(!body_debug.contains("private.example"));
        assert!(body_debug.contains("upload_detail_count: 1"));
    }
}
