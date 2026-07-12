//! Typed responses for document layout parsing and web-page reading.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{
    ZaiResult,
    client::error::{ZaiError, codes},
};

fn response_error(message: impl Into<String>) -> ZaiError {
    ZaiError::ApiError {
        code: codes::SDK_VALIDATION,
        message: message.into(),
    }
}

/// Element category in a parsed document layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LayoutLabel {
    /// Image element.
    #[serde(rename = "image")]
    Image,
    /// Text element.
    #[serde(rename = "text")]
    Text,
    /// Display-formula element.
    #[serde(rename = "formula")]
    Formula,
    /// Table element.
    #[serde(rename = "table")]
    Table,
}

/// One element in a parsed document layout.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayoutDetail {
    /// Element index.
    pub index: i64,
    /// Element category.
    pub label: LayoutLabel,
    /// Normalized `[x1, y1, x2, y2]` coordinates.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bbox_2d: Option<[f64; 4]>,
    /// Text, image URL, formula, or table HTML.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    /// Source page height.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<i64>,
    /// Source page width.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<i64>,
}

/// Dimensions for one parsed page.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PageInfo {
    /// Page width.
    pub width: i64,
    /// Page height.
    pub height: i64,
}

/// Basic information about a parsed document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DataInfo {
    /// Number of pages in the document.
    pub num_pages: i64,
    /// Per-page dimensions, when returned.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pages: Option<Vec<PageInfo>>,
}

/// Cached-token details returned for layout parsing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayoutPromptTokensDetails {
    /// Number of cached prompt tokens.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cached_tokens: Option<f64>,
}

/// Token usage returned by layout parsing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayoutUsage {
    /// Prompt token count.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_tokens: Option<f64>,
    /// Completion token count.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completion_tokens: Option<f64>,
    /// Cached-token details.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_tokens_details: Option<LayoutPromptTokensDetails>,
    /// Total token count.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_tokens: Option<i64>,
}

/// Response from the layout-parsing endpoint.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayoutParsingResponse {
    /// Task identifier.
    pub id: String,
    /// Creation time as a Unix timestamp.
    pub created: i64,
    /// Model name returned by the service.
    pub model: String,
    /// Markdown parsing result.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub md_results: Option<String>,
    /// Layout elements grouped by page.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub layout_details: Option<Vec<Vec<LayoutDetail>>>,
    /// Layout-visualization image URLs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub layout_visualization: Option<Vec<String>>,
    /// Document information.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_info: Option<DataInfo>,
    /// Token usage.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<LayoutUsage>,
    /// Request identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
}

impl LayoutParsingResponse {
    /// Validate normalized bounding-box coordinates from the response.
    pub fn validate(&self) -> ZaiResult<()> {
        let invalid_bbox = self
            .layout_details
            .iter()
            .flatten()
            .flatten()
            .filter_map(|detail| detail.bbox_2d)
            .flatten()
            .any(|coordinate| !coordinate.is_finite() || !(0.0..=1.0).contains(&coordinate));
        if invalid_bbox {
            return Err(response_error(
                "layout response bbox_2d coordinates must be finite values in 0..=1",
            ));
        }
        Ok(())
    }
}

/// One external stylesheet referenced by a reader result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReaderStylesheet {
    /// Stylesheet MIME type.
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub type_: Option<String>,
}

/// External resources referenced by a reader result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReaderExternalResources {
    /// Stylesheets keyed by the upstream resource identifier. This is the sole
    /// `additionalProperties` map in the frozen Reader response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stylesheet: Option<BTreeMap<String, ReaderStylesheet>>,
}

/// Page metadata returned by the reader endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReaderMetadata {
    /// Page keywords.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keywords: Option<String>,
    /// Viewport metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub viewport: Option<String>,
    /// Page description metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Format-detection metadata.
    #[serde(rename = "format-detection", skip_serializing_if = "Option::is_none")]
    pub format_detection: Option<String>,
}

/// Parsed web-page content returned by the reader endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReaderResult {
    /// Parsed page content.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    /// Page description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Page title.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Original page URL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// External page resources.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external: Option<ReaderExternalResources>,
    /// Page metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<ReaderMetadata>,
}

/// Response from the reader endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReaderResponse {
    /// Task identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Creation time as a Unix timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created: Option<i64>,
    /// Request identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    /// Model identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Parsed page result.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reader_result: Option<ReaderResult>,
}

impl ReaderResponse {
    /// Enforce the frozen operation's non-empty success-response invariant.
    pub fn validate(&self) -> ZaiResult<()> {
        if self.id.is_some()
            || self.created.is_some()
            || self.request_id.is_some()
            || self.model.is_some()
            || self.reader_result.is_some()
        {
            return Ok(());
        }
        Err(response_error(
            "reader response contained no documented fields",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_required_fields_and_fixed_bbox_length_are_enforced() {
        assert!(
            serde_json::from_value::<LayoutParsingResponse>(serde_json::json!({
                "created": 1,
                "model": "GLM-OCR"
            }))
            .is_err()
        );
        assert!(
            serde_json::from_value::<LayoutParsingResponse>(serde_json::json!({
                "id": "task-1",
                "created": 1,
                "model": "GLM-OCR",
                "layout_details": [[{"index": 0, "label": "text", "bbox_2d": [0.0, 1.0]}]]
            }))
            .is_err()
        );
        assert!(serde_json::from_value::<LayoutDetail>(serde_json::json!({"index": 1})).is_err());
        assert!(serde_json::from_value::<DataInfo>(serde_json::json!({"pages": []})).is_err());
        assert!(serde_json::from_value::<PageInfo>(serde_json::json!({"width": 1})).is_err());
    }

    #[test]
    fn layout_validation_rejects_out_of_range_coordinates() {
        let response: LayoutParsingResponse = serde_json::from_value(serde_json::json!({
            "id": "task-1",
            "created": 1,
            "model": "GLM-OCR",
            "layout_details": [[{
                "index": 0,
                "label": "text",
                "bbox_2d": [0.0, 0.0, 1.2, 1.0]
            }]]
        }))
        .unwrap();
        assert!(response.validate().is_err());
    }

    #[test]
    fn reader_response_models_typed_additional_properties() {
        let response: ReaderResponse = serde_json::from_value(serde_json::json!({
            "reader_result": {
                "external": {"stylesheet": {"main": {"type": "text/css"}}},
                "metadata": {"format-detection": "telephone=no"}
            }
        }))
        .unwrap();
        assert_eq!(
            response
                .reader_result
                .as_ref()
                .unwrap()
                .external
                .as_ref()
                .unwrap()
                .stylesheet
                .as_ref()
                .unwrap()["main"]
                .type_
                .as_deref(),
            Some("text/css")
        );
        assert!(response.validate().is_ok());
        assert!(
            serde_json::from_value::<ReaderResponse>(serde_json::json!({}))
                .unwrap()
                .validate()
                .is_err()
        );
    }
}
