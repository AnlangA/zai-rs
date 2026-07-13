//! Typed requests for document layout parsing and web-page reading.

use serde::Serialize;

use crate::{
    ZaiResult,
    client::{
        ZaiClient,
        error::{ZaiError, codes},
    },
};

use super::response::{LayoutParsingResponse, ReaderResponse};

fn validation_error(message: impl Into<String>) -> ZaiError {
    ZaiError::ApiError {
        code: codes::SDK_VALIDATION,
        message: message.into(),
    }
}

fn invalid_optional_length(value: Option<&str>, min: usize, max: usize) -> bool {
    value.is_some_and(|value| !(min..=max).contains(&value.chars().count()))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
enum LayoutParsingModel {
    #[serde(rename = "glm-ocr")]
    GlmOcr,
}

/// Request for `POST /layout_parsing`.
#[derive(Clone, Serialize)]
pub struct LayoutParsingRequest {
    model: LayoutParsingModel,
    file: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    return_crop_images: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    need_layout_visualization: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    start_page_id: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    end_page_id: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    request_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    user_id: Option<String>,
}

impl std::fmt::Debug for LayoutParsingRequest {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("LayoutParsingRequest")
            .field("model", &self.model())
            .field("file", &"[REDACTED]")
            .field("return_crop_images", &self.return_crop_images)
            .field("need_layout_visualization", &self.need_layout_visualization)
            .field("start_page_id", &self.start_page_id)
            .field("end_page_id", &self.end_page_id)
            .field(
                "request_id",
                &self.request_id.as_ref().map(|_| "[REDACTED]"),
            )
            .field("user_id", &self.user_id.as_ref().map(|_| "[REDACTED]"))
            .finish()
    }
}

impl LayoutParsingRequest {
    /// Create a request for a URL or base64-encoded PDF/JPG/PNG input.
    pub fn new(file: impl Into<String>) -> Self {
        Self {
            model: LayoutParsingModel::GlmOcr,
            file: file.into(),
            return_crop_images: None,
            need_layout_visualization: None,
            start_page_id: None,
            end_page_id: None,
            request_id: None,
            user_id: None,
        }
    }

    /// Configure cropped-image output.
    pub fn with_crop_images(mut self, enabled: bool) -> Self {
        self.return_crop_images = Some(enabled);
        self
    }

    /// Configure layout-visualization output.
    pub fn with_layout_visualization(mut self, enabled: bool) -> Self {
        self.need_layout_visualization = Some(enabled);
        self
    }

    /// Select the inclusive PDF page range.
    pub fn with_page_range(mut self, start: u32, end: u32) -> Self {
        self.start_page_id = Some(start);
        self.end_page_id = Some(end);
        self
    }

    /// Set only the first PDF page to parse.
    pub fn with_start_page_id(mut self, start: u32) -> Self {
        self.start_page_id = Some(start);
        self
    }

    /// Set only the last PDF page to parse.
    pub fn with_end_page_id(mut self, end: u32) -> Self {
        self.end_page_id = Some(end);
        self
    }

    /// Set the caller-provided request identifier.
    pub fn with_request_id(mut self, request_id: impl Into<String>) -> Self {
        self.request_id = Some(request_id.into());
        self
    }

    /// Set the end-user identifier used for abuse monitoring.
    pub fn with_user_id(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = Some(user_id.into());
        self
    }

    /// Return the fixed model identifier sent on the wire.
    pub const fn model(&self) -> &'static str {
        "glm-ocr"
    }

    /// Borrow the URL or base64 input.
    pub fn file(&self) -> &str {
        &self.file
    }

    /// Return the selected PDF page range.
    pub const fn page_range(&self) -> Option<(u32, u32)> {
        match (self.start_page_id, self.end_page_id) {
            (Some(start), Some(end)) => Some((start, end)),
            _ => None,
        }
    }

    /// Return the optional first PDF page.
    pub const fn start_page_id(&self) -> Option<u32> {
        self.start_page_id
    }

    /// Return the optional last PDF page.
    pub const fn end_page_id(&self) -> Option<u32> {
        self.end_page_id
    }

    /// Return the crop-image flag.
    pub const fn crop_images_enabled(&self) -> Option<bool> {
        self.return_crop_images
    }

    /// Return the layout-visualization flag.
    pub const fn layout_visualization_enabled(&self) -> Option<bool> {
        self.need_layout_visualization
    }

    /// Borrow the caller-provided request identifier.
    pub fn request_id(&self) -> Option<&str> {
        self.request_id.as_deref()
    }

    /// Borrow the end-user identifier.
    pub fn user_id(&self) -> Option<&str> {
        self.user_id.as_deref()
    }

    /// Validate frozen OpenAPI numeric and identifier constraints.
    pub fn validate(&self) -> ZaiResult<()> {
        if self.file.trim().is_empty() {
            return Err(validation_error("layout parsing file must not be blank"));
        }
        if self.start_page_id == Some(0) || self.end_page_id == Some(0) {
            return Err(validation_error("layout page ids must be at least 1"));
        }
        if matches!(
            (self.start_page_id, self.end_page_id),
            (Some(start), Some(end)) if start > end
        ) {
            return Err(validation_error(
                "layout start_page_id must not exceed end_page_id",
            ));
        }
        if invalid_optional_length(self.request_id.as_deref(), 6, 64) {
            return Err(validation_error(
                "layout request_id must contain between 6 and 64 characters",
            ));
        }
        if invalid_optional_length(self.user_id.as_deref(), 6, 128) {
            return Err(validation_error(
                "layout user_id must contain between 6 and 128 characters",
            ));
        }
        Ok(())
    }

    /// Validate, send, and decode a layout-parsing response.
    pub async fn send_via(&self, client: &ZaiClient) -> ZaiResult<LayoutParsingResponse> {
        self.validate()?;
        let route = crate::client::routes::TOOLS_LAYOUT;
        let response = client
            .operation(route)
            .send_json::<_, LayoutParsingResponse>(self)
            .await?;
        response.validate()?;
        Ok(response)
    }
}

/// Request for `POST /reader`.
#[derive(Clone, Serialize)]
pub struct ReaderRequest {
    url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    timeout: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    no_cache: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    return_format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    retain_images: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    no_gfm: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    keep_img_data_url: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    with_images_summary: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    with_links_summary: Option<bool>,
}

impl std::fmt::Debug for ReaderRequest {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ReaderRequest")
            .field("url", &"[REDACTED]")
            .field("timeout", &self.timeout)
            .field("no_cache", &self.no_cache)
            .field("return_format", &self.return_format)
            .field("retain_images", &self.retain_images)
            .field("no_gfm", &self.no_gfm)
            .field("keep_img_data_url", &self.keep_img_data_url)
            .field("with_images_summary", &self.with_images_summary)
            .field("with_links_summary", &self.with_links_summary)
            .finish()
    }
}

impl ReaderRequest {
    /// Create a reader request for the required URL.
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            timeout: None,
            no_cache: None,
            return_format: None,
            retain_images: None,
            no_gfm: None,
            keep_img_data_url: None,
            with_images_summary: None,
            with_links_summary: None,
        }
    }

    /// Set the upstream request timeout in seconds.
    pub fn with_timeout_seconds(mut self, timeout: u64) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Configure cache bypass.
    pub fn with_cache_disabled(mut self, disabled: bool) -> Self {
        self.no_cache = Some(disabled);
        self
    }

    /// Set the requested output format, such as `markdown` or `text`.
    pub fn with_return_format(mut self, format: impl Into<String>) -> Self {
        self.return_format = Some(format.into());
        self
    }

    /// Configure whether images remain in the parsed content.
    pub fn with_retained_images(mut self, retain: bool) -> Self {
        self.retain_images = Some(retain);
        self
    }

    /// Configure GitHub-Flavored Markdown conversion.
    pub fn with_gfm_disabled(mut self, disabled: bool) -> Self {
        self.no_gfm = Some(disabled);
        self
    }

    /// Configure whether image data URLs are preserved.
    pub fn with_image_data_urls(mut self, keep: bool) -> Self {
        self.keep_img_data_url = Some(keep);
        self
    }

    /// Configure image summaries.
    pub fn with_image_summaries(mut self, enabled: bool) -> Self {
        self.with_images_summary = Some(enabled);
        self
    }

    /// Configure link summaries.
    pub fn with_link_summaries(mut self, enabled: bool) -> Self {
        self.with_links_summary = Some(enabled);
        self
    }

    /// Borrow the source URL.
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Return the configured timeout in seconds.
    pub const fn timeout_seconds(&self) -> Option<u64> {
        self.timeout
    }

    /// Borrow the requested return format.
    pub fn return_format(&self) -> Option<&str> {
        self.return_format.as_deref()
    }

    /// Return whether cache bypass was configured.
    pub const fn cache_disabled(&self) -> Option<bool> {
        self.no_cache
    }

    /// Return whether parsed images are retained.
    pub const fn retained_images(&self) -> Option<bool> {
        self.retain_images
    }

    /// Return whether GitHub-Flavored Markdown is disabled.
    pub const fn gfm_disabled(&self) -> Option<bool> {
        self.no_gfm
    }

    /// Return whether image data URLs are preserved.
    pub const fn keeps_image_data_urls(&self) -> Option<bool> {
        self.keep_img_data_url
    }

    /// Return whether image summaries are requested.
    pub const fn image_summaries_enabled(&self) -> Option<bool> {
        self.with_images_summary
    }

    /// Return whether link summaries are requested.
    pub const fn link_summaries_enabled(&self) -> Option<bool> {
        self.with_links_summary
    }

    /// Validate the required URL string.
    pub fn validate(&self) -> ZaiResult<()> {
        if self.url.trim().is_empty() {
            return Err(validation_error("reader url must not be blank"));
        }
        Ok(())
    }

    /// Validate, send, and decode a reader response.
    pub async fn send_via(&self, client: &ZaiClient) -> ZaiResult<ReaderResponse> {
        self.validate()?;
        let route = crate::client::routes::TOOLS_READER;
        let response = client
            .operation(route)
            .send_json::<_, ReaderResponse>(self)
            .await?;
        response.validate()?;
        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_request_serializes_fixed_model_and_exact_wire_names() {
        let request = LayoutParsingRequest::new("https://example.test/document.pdf")
            .with_crop_images(true)
            .with_layout_visualization(false)
            .with_page_range(2, 4)
            .with_request_id("request-1")
            .with_user_id("user-1");
        assert_eq!(
            serde_json::to_value(request).unwrap(),
            serde_json::json!({
                "model": "glm-ocr",
                "file": "https://example.test/document.pdf",
                "return_crop_images": true,
                "need_layout_visualization": false,
                "start_page_id": 2,
                "end_page_id": 4,
                "request_id": "request-1",
                "user_id": "user-1"
            })
        );
        let debug = format!(
            "{:?}",
            LayoutParsingRequest::new("private-file-data")
                .with_request_id("private-request")
                .with_user_id("private-user")
        );
        assert!(!debug.contains("private-file-data"));
        assert!(!debug.contains("private-request"));
        assert!(!debug.contains("private-user"));
    }

    #[test]
    fn request_validation_enforces_documented_constraints() {
        assert!(LayoutParsingRequest::new(" ").validate().is_err());
        assert!(
            LayoutParsingRequest::new("data")
                .with_page_range(2, 1)
                .validate()
                .is_err()
        );
        assert!(
            LayoutParsingRequest::new("data")
                .with_request_id("short")
                .validate()
                .is_err()
        );
        assert!(ReaderRequest::new(" ").validate().is_err());
    }

    #[test]
    fn reader_request_serializes_only_selected_closed_fields() {
        let request = ReaderRequest::new("https://example.test/page")
            .with_timeout_seconds(30)
            .with_cache_disabled(true)
            .with_return_format("markdown")
            .with_retained_images(true)
            .with_image_summaries(true);
        assert_eq!(
            serde_json::to_value(request).unwrap(),
            serde_json::json!({
                "url": "https://example.test/page",
                "timeout": 30,
                "no_cache": true,
                "return_format": "markdown",
                "retain_images": true,
                "with_images_summary": true
            })
        );
        assert!(
            !format!("{:?}", ReaderRequest::new("https://secret.test/token"))
                .contains("secret.test")
        );
    }
}
