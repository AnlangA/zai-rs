//! Typed response models for the LLM-application endpoints.
//!
//! The structures in this module mirror the frozen OpenAPI schemas. Required
//! response fields deliberately do not use serde defaults: a malformed success
//! payload therefore fails during deserialization instead of silently becoming
//! an empty value.

use serde::{Deserialize, Serialize};

use crate::{
    ZaiResult,
    client::error::{ZaiError, codes},
};

/// Standard success envelope returned by the application-v2 endpoints.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApplicationEnvelope<T> {
    /// Endpoint-specific response payload.
    pub data: T,
    /// Business status code (`200` indicates success).
    pub code: i64,
    /// Human-readable business status message.
    pub message: String,
    /// Server response timestamp.
    pub timestamp: i64,
}

/// Parsing status for one application file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicationFileStatItem {
    /// File identifier.
    pub file_id: String,
    /// Document parsing status code.
    pub code: i64,
    /// Human-readable parsing status.
    pub msg: String,
}

/// Response from the application file-statistics endpoint.
pub type ApplicationFileStatsResponse = ApplicationEnvelope<Vec<ApplicationFileStatItem>>;

/// Successfully uploaded application file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicationFileUploadSuccessInfo {
    /// Identifier assigned to the uploaded file.
    pub file_id: String,
    /// Original file name.
    pub file_name: String,
}

/// Application file that failed to upload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicationFileUploadFailInfo {
    /// Original file name.
    pub file_name: String,
    /// Server-provided failure reason.
    pub fail_reason: String,
}

/// File-level results returned by an application upload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicationFileUploadData {
    /// Files uploaded successfully.
    pub success_info: Vec<ApplicationFileUploadSuccessInfo>,
    /// Files rejected by the service.
    pub fail_info: Vec<ApplicationFileUploadFailInfo>,
}

/// Response from the application file-upload endpoint.
pub type ApplicationFileUploadResponse = ApplicationEnvelope<ApplicationFileUploadData>;

/// Metadata for a document represented in slice information.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicationDocumentInfo {
    /// Document identifier.
    pub id: String,
    /// Document name.
    pub name: String,
    /// Document URL.
    pub url: String,
    /// Upstream numeric document type.
    pub dtype: i64,
}

/// Page-space coordinates for a document slice.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApplicationSlicePosition {
    /// Left edge of the slice.
    pub x0: f32,
    /// Right edge of the slice.
    pub x1: f32,
    /// Top edge of the slice.
    pub top: f32,
    /// Bottom edge of the slice.
    pub bottom: f32,
    /// One-based page number supplied by the service.
    pub page: i64,
    /// Source page height.
    pub height: f32,
    /// Source page width.
    pub width: f32,
}

/// One text slice extracted from a document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApplicationSliceInfo {
    /// Source document identifier.
    pub document_id: String,
    /// Page-space position, when the document format provides it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<ApplicationSlicePosition>,
    /// Spreadsheet row number, when applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<i64>,
    /// Spreadsheet sheet name, when applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sheet_name: Option<String>,
    /// Extracted slice text.
    pub text: String,
}

/// Image associated with a document slice.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicationSliceImage {
    /// Image name or source text.
    pub text: String,
    /// Image object-storage URL.
    pub cos_url: String,
}

/// Slice information grouped by source document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApplicationDocumentSlices {
    /// Source document metadata.
    pub document: ApplicationDocumentInfo,
    /// Text slices extracted from the document.
    pub slice_info: Vec<ApplicationSliceInfo>,
    /// Whether historical slices lack position information.
    pub hide_positions: bool,
    /// Images extracted from the document, when returned.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub images: Option<Vec<ApplicationSliceImage>>,
}

/// Payload returned by the application slice-information endpoint.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApplicationSliceInfoData {
    /// Slice groups for each document.
    pub document_slices: Vec<ApplicationDocumentSlices>,
    /// Whether an older document without slice positions exists.
    pub has_old_document: bool,
}

/// Response from the application slice-information endpoint.
pub type ApplicationSliceInfoResponse = ApplicationEnvelope<ApplicationSliceInfoData>;

/// Payload returned after creating an application conversation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicationConversationCreateData {
    /// Newly created conversation identifier.
    pub conversation_id: String,
}

/// Response returned after creating an application conversation.
pub type ApplicationConversationCreateResponse =
    ApplicationEnvelope<ApplicationConversationCreateData>;

/// Input-template metadata attached to an application variable.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApplicationInputTemplate {
    /// Open-schema option values from the upstream API.
    pub options: Vec<serde_json::Value>,
}

/// One input variable exposed by an application.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApplicationVariable {
    /// Variable identifier.
    pub id: String,
    /// Variable name.
    pub name: String,
    /// Upstream variable type.
    #[serde(rename = "type")]
    pub type_: String,
    /// User-facing input hint.
    pub tips: String,
    /// Allowed selection values.
    pub allowed_values: Vec<String>,
    /// Input template supplied by the application.
    pub input_template: ApplicationInputTemplate,
}

/// Response containing an application's input variables.
pub type ApplicationVariablesResponse = ApplicationEnvelope<Vec<ApplicationVariable>>;

/// Recommended questions returned for an application conversation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicationHistoryData {
    /// Recommended questions.
    pub problems: Vec<String>,
}

/// Response containing application conversation recommendations.
pub type ApplicationHistoryResponse = ApplicationEnvelope<ApplicationHistoryData>;

/// Content emitted by an application invocation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplicationMessageData {
    /// Text, reasoning, image, video, or tool output.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub msg: Option<String>,
    /// Output type such as `text`, `image`, `video`, or `all_tools`.
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub type_: Option<String>,
    /// Generated code for an all-tools result.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    /// Generated file URL for an all-tools result.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    /// Generated image or video URL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Generated video cover URL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover_url: Option<String>,
}

/// Tool-call metadata emitted by an application node.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApplicationToolCallsData {
    /// Tool category such as `function`, `retrieval`, or `web_search`.
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub type_: Option<String>,
    /// Open object containing tool-specific message or log fields.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls_data: Option<serde_json::Map<String, serde_json::Value>>,
}

/// Execution event emitted by an application node.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApplicationInvokeEvent {
    /// Node identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_id: Option<String>,
    /// Node name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_name: Option<String>,
    /// Event type.
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub type_: Option<String>,
    /// Event content.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    /// Event duration or timestamp in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time: Option<i64>,
    /// Tool-call metadata associated with the event.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<ApplicationToolCallsData>,
}

/// Incremental output returned in one invocation choice.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApplicationInvokeDelta {
    /// Incremental model output.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<ApplicationMessageData>,
    /// Node execution event.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event: Option<ApplicationInvokeEvent>,
    /// Tool-call metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<ApplicationToolCallsData>,
}

/// Aggregated output returned in one invocation choice.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApplicationInvokeMessages {
    /// Aggregated model output.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<ApplicationMessageData>,
    /// Node execution events.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event: Option<Vec<ApplicationInvokeEvent>>,
}

/// Structured error attached to a failed invocation choice.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicationInvokeError {
    /// Application error code.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<i64>,
    /// Application error message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub msg: Option<String>,
}

/// One result choice returned by an application invocation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApplicationInvokeChoice {
    /// Choice index.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<i64>,
    /// Completion reason such as `stop` or `error`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
    /// Error details for a failed choice.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_msg: Option<ApplicationInvokeError>,
    /// Incremental output fields.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delta: Option<ApplicationInvokeDelta>,
    /// Aggregated output fields.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub messages: Option<ApplicationInvokeMessages>,
}

/// Token usage for one model node in an application invocation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplicationInvokeUsage {
    /// Model used by the node.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Node name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_name: Option<String>,
    /// Input token count.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_token_count: Option<i64>,
    /// Output token count.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_token_count: Option<i64>,
    /// Total token count.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_token_count: Option<i64>,
}

/// Response from an application-v3 invocation.
///
/// The frozen OpenAPI schema marks every individual field optional. The
/// operations contract still requires at least one documented field to be
/// present; [`validate`](Self::validate) enforces that invariant after decode.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApplicationInvokeResponse {
    /// Request identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    /// Conversation identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conversation_id: Option<String>,
    /// Application identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app_id: Option<String>,
    /// Invocation choices.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub choices: Option<Vec<ApplicationInvokeChoice>>,
    /// Per-node token usage.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<Vec<ApplicationInvokeUsage>>,
}

impl ApplicationInvokeResponse {
    /// Enforce the frozen operation's non-empty success-response invariant.
    pub fn validate(&self) -> ZaiResult<()> {
        if self.request_id.is_some()
            || self.conversation_id.is_some()
            || self.app_id.is_some()
            || self.choices.is_some()
            || self.usage.is_some()
        {
            return Ok(());
        }

        Err(ZaiError::ApiError {
            code: codes::SDK_VALIDATION,
            message: "application invoke response contained no documented fields".to_owned(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn required_application_envelope_fields_do_not_default() {
        let missing_code =
            serde_json::from_value::<ApplicationHistoryResponse>(serde_json::json!({
                "data": {"problems": []},
                "message": "ok",
                "timestamp": 1
            }));
        assert!(missing_code.is_err());
    }

    #[test]
    fn invocation_uses_the_official_top_level_and_camel_case_fields() {
        let response: ApplicationInvokeResponse = serde_json::from_value(serde_json::json!({
            "request_id": "req-1",
            "choices": [{
                "index": 0,
                "delta": {"content": {"type": "video", "coverUrl": "https://cover"}}
            }],
            "usage": [{"nodeName": "model", "totalTokenCount": 3}]
        }))
        .unwrap();

        assert_eq!(response.request_id.as_deref(), Some("req-1"));
        let choice = &response.choices.as_ref().unwrap()[0];
        assert_eq!(
            choice
                .delta
                .as_ref()
                .unwrap()
                .content
                .as_ref()
                .unwrap()
                .cover_url
                .as_deref(),
            Some("https://cover")
        );
        assert_eq!(
            response.usage.as_ref().unwrap()[0].total_token_count,
            Some(3)
        );
        assert!(response.validate().is_ok());
        assert!(
            serde_json::from_value::<ApplicationInvokeResponse>(serde_json::json!({}))
                .unwrap()
                .validate()
                .is_err()
        );
    }
}
