//! Typed response models for assistant invocation and discovery endpoints.

use serde::{Deserialize, Serialize};

use crate::{
    ZaiResult,
    client::error::{ZaiError, codes},
};

fn empty_response(operation: &str) -> ZaiError {
    ZaiError::ApiError {
        code: codes::SDK_VALIDATION,
        message: format!("{operation} response contained no documented fields"),
    }
}

/// Text content inside a multimodal assistant response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssistantResponseContentPart {
    /// Content type. The current response schema permits only `text`.
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub type_: Option<AssistantResponseContentType>,
    /// Text content.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

/// Content type used by a multimodal assistant response part.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssistantResponseContentType {
    /// A text response part.
    #[serde(rename = "text")]
    Text,
}

/// Text or multimodal content returned by an assistant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AssistantResponseContent {
    /// Plain text content.
    Text(String),
    /// Multimodal response parts.
    Parts(Vec<AssistantResponseContentPart>),
}

/// Audio content returned by a voice-capable assistant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssistantResponseAudio {
    /// Audio identifier used for follow-up turns.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Base64-encoded audio data.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
    /// Audio expiry timestamp as returned by the API.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
}

/// Function call returned by an assistant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssistantResponseFunctionCall {
    /// Function name.
    pub name: String,
    /// JSON-formatted function arguments.
    pub arguments: String,
}

/// MCP call type returned by an assistant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssistantMcpCallType {
    /// List the server's available tools.
    #[serde(rename = "mcp_list_tools")]
    ListTools,
    /// Invoke one MCP tool.
    #[serde(rename = "mcp_call")]
    Call,
}

/// JSON-schema type used by an MCP input schema.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssistantMcpSchemaType {
    /// JSON object schema.
    #[serde(rename = "object")]
    Object,
}

/// Input schema advertised by an MCP tool.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssistantMcpInputSchema {
    /// Schema type (`object`).
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub type_: Option<AssistantMcpSchemaType>,
    /// Open map of property definitions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<serde_json::Map<String, serde_json::Value>>,
    /// Required property names.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<Vec<String>>,
    /// Whether undeclared properties are accepted.
    #[serde(
        rename = "additionalProperties",
        skip_serializing_if = "Option::is_none"
    )]
    pub additional_properties: Option<bool>,
}

/// Tool descriptor returned by an MCP list-tools call.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssistantMcpTool {
    /// Tool name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Tool description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Open annotation object.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<serde_json::Map<String, serde_json::Value>>,
    /// Tool input schema.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_schema: Option<AssistantMcpInputSchema>,
}

/// MCP payload attached to an assistant tool call.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssistantMcpCall {
    /// MCP call identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// MCP operation type.
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub type_: Option<AssistantMcpCallType>,
    /// MCP server label.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_label: Option<String>,
    /// Error returned by the MCP server.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Tools returned by a list-tools operation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<AssistantMcpTool>>,
    /// JSON-formatted call arguments.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<String>,
    /// Called tool name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Open object returned by the MCP tool.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<serde_json::Map<String, serde_json::Value>>,
}

/// Tool call returned in an assistant response message.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssistantResponseToolCall {
    /// Function-call payload.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function: Option<AssistantResponseFunctionCall>,
    /// MCP-call payload.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mcp: Option<AssistantMcpCall>,
    /// Tool-call identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Tool type such as `function` or `mcp`.
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub type_: Option<String>,
}

/// Message returned in an assistant invocation choice.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssistantResponseMessage {
    /// Response role.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    /// Text or multimodal response content. JSON `null` maps to `None`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<AssistantResponseContent>,
    /// Model reasoning content, when returned.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_content: Option<String>,
    /// Voice-model audio payload.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio: Option<AssistantResponseAudio>,
    /// Function or MCP tool calls.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<AssistantResponseToolCall>>,
}

/// One assistant invocation choice.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssistantInvokeChoice {
    /// Choice index.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<i64>,
    /// Assistant message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<AssistantResponseMessage>,
    /// Completion reason.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
}

/// Token usage returned by an assistant invocation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssistantUsage {
    /// Prompt token count.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_tokens: Option<i64>,
    /// Completion token count.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completion_tokens: Option<i64>,
    /// Total token count.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_tokens: Option<i64>,
}

/// Response from invoking an assistant.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssistantInvokeResponse {
    /// Invocation identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Request identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    /// Creation timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created: Option<i64>,
    /// Model used by the assistant.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Assistant choices.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub choices: Option<Vec<AssistantInvokeChoice>>,
    /// Token usage.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<AssistantUsage>,
}

impl AssistantInvokeResponse {
    /// Enforce the operation contract's non-empty success-response invariant.
    pub fn validate(&self) -> ZaiResult<()> {
        if self.id.is_some()
            || self.request_id.is_some()
            || self.created.is_some()
            || self.model.is_some()
            || self.choices.is_some()
            || self.usage.is_some()
        {
            Ok(())
        } else {
            Err(empty_response("assistant invoke"))
        }
    }
}

/// One key/value tag attached to an assistant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssistantTag {
    /// Tag key.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    /// Human-readable tag label.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

/// Assistant record returned by the list endpoint.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssistantInfo {
    /// Assistant identifier.
    pub assistant_id: String,
    /// Assistant name.
    pub name: String,
    /// Avatar URL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar: Option<String>,
    /// Assistant description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Supported tool names.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<String>>,
    /// Assistant tags.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<AssistantTag>>,
    /// Assistant status.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    /// Open starter-prompt objects.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub starter_prompts: Option<Vec<serde_json::Map<String, serde_json::Value>>>,
    /// Creation time.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    /// Last update time.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
}

/// Response containing assistants available to the current account.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssistantListResponse {
    /// Whether the operation succeeded.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub success: Option<bool>,
    /// Business status code.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<i64>,
    /// Business status message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub msg: Option<String>,
    /// Matching assistants.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Vec<AssistantInfo>>,
}

impl AssistantListResponse {
    /// Enforce the operation contract's non-empty success-response invariant.
    pub fn validate(&self) -> ZaiResult<()> {
        if self.success.is_some()
            || self.code.is_some()
            || self.msg.is_some()
            || self.data.is_some()
        {
            Ok(())
        } else {
            Err(empty_response("assistant list"))
        }
    }
}

/// Token usage recorded for one assistant conversation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssistantConversationUsage {
    /// Prompt token count.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_tokens: Option<i64>,
    /// Completion token count.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completion_tokens: Option<i64>,
    /// Total token count.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_tokens: Option<i64>,
}

/// One assistant conversation record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssistantConversation {
    /// Conversation identifier.
    pub id: String,
    /// Assistant identifier.
    pub assistant_id: String,
    /// Creation time.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub create_time: Option<String>,
    /// Last update time.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub update_time: Option<String>,
    /// Conversation token usage.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<AssistantConversationUsage>,
}

/// Paginated assistant-conversation payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssistantConversationPage {
    /// Assistant identifier.
    pub assistant_id: String,
    /// Conversations on this page.
    pub conversation_list: Vec<AssistantConversation>,
    /// Whether another page is available.
    pub has_more: bool,
}

/// Response containing an assistant's conversations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssistantConversationListResponse {
    /// Whether the operation succeeded.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub success: Option<bool>,
    /// Business status code.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<i64>,
    /// Business status message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub msg: Option<String>,
    /// Paginated conversation data.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<AssistantConversationPage>,
}

impl AssistantConversationListResponse {
    /// Enforce the operation contract's non-empty success-response invariant.
    pub fn validate(&self) -> ZaiResult<()> {
        if self.success.is_some()
            || self.code.is_some()
            || self.msg.is_some()
            || self.data.is_some()
        {
            Ok(())
        } else {
            Err(empty_response("assistant conversation list"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_items_preserve_required_fields() {
        let response: AssistantListResponse = serde_json::from_value(serde_json::json!({
            "success": true,
            "code": 200,
            "msg": "ok",
            "data": [{"assistant_id": "assistant-1", "name": "Helper"}]
        }))
        .unwrap();
        assert_eq!(
            response.data.as_ref().unwrap()[0].assistant_id,
            "assistant-1"
        );
        assert!(response.validate().is_ok());

        let missing_name = serde_json::from_value::<AssistantListResponse>(serde_json::json!({
            "data": [{"assistant_id": "assistant-1"}]
        }));
        assert!(missing_name.is_err());
    }

    #[test]
    fn empty_top_level_responses_violate_the_operations_contract() {
        assert!(
            serde_json::from_value::<AssistantInvokeResponse>(serde_json::json!({}))
                .unwrap()
                .validate()
                .is_err()
        );
        assert!(
            serde_json::from_value::<AssistantListResponse>(serde_json::json!({}))
                .unwrap()
                .validate()
                .is_err()
        );
        assert!(
            serde_json::from_value::<AssistantConversationListResponse>(serde_json::json!({}))
                .unwrap()
                .validate()
                .is_err()
        );
    }
}
