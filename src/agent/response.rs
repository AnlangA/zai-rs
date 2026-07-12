//! Exact Agent v1 response contracts.

use serde::{Deserialize, Deserializer, Serialize, de::Error as _};

use crate::{
    ZaiError, ZaiResult,
    client::error::{codes, mask_sensitive_info},
};

fn invalid_response(message: impl Into<String>) -> ZaiError {
    ZaiError::ApiError {
        code: codes::SDK_VALIDATION,
        message: format!(
            "agent response invariant violated: {}",
            mask_sensitive_info(&message.into())
        ),
    }
}

fn required_text(value: Option<String>, field: &str) -> ZaiResult<String> {
    match value {
        Some(value) if !value.trim().is_empty() => Ok(value),
        _ => Err(invalid_response(format!("{field} must be non-blank"))),
    }
}

/// Content-part kinds in a non-streaming Agent invocation response.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentResponseContentType {
    /// Text content.
    Text,
    /// Downloadable file URL.
    FileUrl,
    /// Downloadable image URL.
    ImageUrl,
    /// Downloadable audio URL.
    AudioUrl,
    /// Downloadable video URL.
    VideoUrl,
}

/// One multimodal part in an Agent invocation response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentResponseContentPart {
    /// Frozen content discriminator.
    #[serde(rename = "type")]
    pub type_: AgentResponseContentType,
    /// Text value for `type=text`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// File URL for `type=file_url`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_url: Option<String>,
    /// Image URL for `type=image_url`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_url: Option<String>,
    /// Audio URL for `type=audio_url`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio_url: Option<String>,
    /// Video URL for `type=video_url`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub video_url: Option<String>,
}

/// Agent invocation response content in one of the frozen wire forms.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AgentResponseContent {
    /// Plain text response.
    Text(String),
    /// One multimodal response part.
    Part(AgentResponseContentPart),
    /// Multiple multimodal response parts.
    Parts(Vec<AgentResponseContentPart>),
}

/// One generated message in an invocation choice.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentResponseMessage {
    /// Response role, usually `assistant`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    /// Text or multimodal response content.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<AgentResponseContent>,
}

/// One invocation result choice.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentChoice {
    /// Result index.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<i64>,
    /// Generated response messages.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub messages: Option<Vec<AgentResponseMessage>>,
    /// Service-provided finish reason.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
}

/// Token usage returned by a completed Agent invocation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentUsage {
    /// Number of input tokens.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_tokens: Option<i64>,
    /// Number of output tokens.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completion_tokens: Option<i64>,
    /// Total token count.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_tokens: Option<i64>,
}

/// Completed non-streaming Agent invocation.
#[derive(Debug, Clone, Serialize)]
pub struct AgentCompletedResponse {
    /// Request identifier.
    pub id: String,
    /// Agent identifier.
    pub agent_id: String,
    /// Conversation identifier, when assigned.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conversation_id: Option<String>,
    /// Non-empty generated choices.
    pub choices: Vec<AgentChoice>,
    /// Optional token usage.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<AgentUsage>,
}

/// Pending asynchronous Agent invocation.
#[derive(Debug, Clone, Serialize)]
pub struct AgentPendingResponse {
    /// Agent identifier.
    pub agent_id: String,
    /// Identifier used to poll the async-result operation.
    pub async_id: String,
}

/// A non-streaming Agent invocation response.
///
/// The frozen wire schema has no `status` property. Deserialization therefore
/// distinguishes a completed payload from a pending payload using their
/// mutually exclusive documented fields.
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum AgentInvokeResponse {
    /// Completed result with generated choices.
    Completed(AgentCompletedResponse),
    /// Accepted asynchronous invocation.
    Pending(AgentPendingResponse),
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AgentInvokeWire {
    id: Option<String>,
    agent_id: Option<String>,
    conversation_id: Option<String>,
    async_id: Option<String>,
    choices: Option<Vec<AgentChoice>>,
    usage: Option<AgentUsage>,
}

impl AgentInvokeResponse {
    /// Validate invariants even for a value constructed in local code.
    pub fn validate(&self) -> ZaiResult<()> {
        match self {
            Self::Completed(response) => {
                if response.id.trim().is_empty()
                    || response.agent_id.trim().is_empty()
                    || response.choices.is_empty()
                {
                    return Err(invalid_response(
                        "completed invoke requires id, agent_id, and non-empty choices",
                    ));
                }
            },
            Self::Pending(response) => {
                if response.agent_id.trim().is_empty() || response.async_id.trim().is_empty() {
                    return Err(invalid_response(
                        "pending invoke requires agent_id and async_id",
                    ));
                }
            },
        }
        Ok(())
    }
}

impl<'de> Deserialize<'de> for AgentInvokeResponse {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;
        let object = value
            .as_object()
            .ok_or_else(|| D::Error::custom("agent invoke response must be a JSON object"))?;
        let has_completed_field = ["id", "conversation_id", "choices", "usage"]
            .iter()
            .any(|field| object.contains_key(*field));
        let has_pending_field = object.contains_key("async_id");

        if has_completed_field && has_pending_field {
            return Err(D::Error::custom(
                "agent invoke response mixed completed and pending fields",
            ));
        }

        let wire: AgentInvokeWire =
            serde_json::from_value(value).map_err(|error| D::Error::custom(error.to_string()))?;
        let response = if has_completed_field {
            let choices = wire.choices.ok_or_else(|| {
                D::Error::custom("completed agent invoke response omitted choices")
            })?;
            if choices.is_empty() {
                return Err(D::Error::custom(
                    "completed agent invoke response contained empty choices",
                ));
            }
            Self::Completed(AgentCompletedResponse {
                id: required_text(wire.id, "id")
                    .map_err(|error| D::Error::custom(error.to_string()))?,
                agent_id: required_text(wire.agent_id, "agent_id")
                    .map_err(|error| D::Error::custom(error.to_string()))?,
                conversation_id: wire.conversation_id,
                choices,
                usage: wire.usage,
            })
        } else if has_pending_field {
            Self::Pending(AgentPendingResponse {
                agent_id: required_text(wire.agent_id, "agent_id")
                    .map_err(|error| D::Error::custom(error.to_string()))?,
                async_id: required_text(wire.async_id, "async_id")
                    .map_err(|error| D::Error::custom(error.to_string()))?,
            })
        } else {
            return Err(D::Error::custom(
                "agent invoke response matched neither completed nor pending shape",
            ));
        };

        Ok(response)
    }
}

/// One content part returned by the async-result endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentAsyncContentPart {
    /// Provider content type; currently documented as `file_url`.
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub type_: Option<String>,
    /// Download URL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_url: Option<String>,
    /// Chinese description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag_cn: Option<String>,
    /// English description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag_en: Option<String>,
}

/// One generated message returned by the async-result endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentAsyncMessage {
    /// Response role.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    /// Generated file content parts.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<Vec<AgentAsyncContentPart>>,
}

/// One async-result choice.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentAsyncChoice {
    /// Generated response messages.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub messages: Option<Vec<AgentAsyncMessage>>,
}

/// Token usage returned by the async-result endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentAsyncUsage {
    /// Total token count.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_tokens: Option<i64>,
}

/// Closed async-result task status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AgentAsyncStatus {
    /// Task completed successfully.
    Success,
    /// Task reached a failed terminal state.
    Failed,
    /// Task is still running.
    Pending,
}

/// Response from `POST /v1/agents/async-result`.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum AgentAsyncResult {
    /// Task is still processing.
    Pending {
        /// Agent identifier.
        agent_id: String,
        /// Asynchronous task identifier.
        async_id: String,
    },
    /// Task completed with non-empty choices.
    Success {
        /// Agent identifier.
        agent_id: String,
        /// Asynchronous task identifier.
        async_id: String,
        /// Non-empty task choices.
        choices: Vec<AgentAsyncChoice>,
        /// Optional token usage.
        #[serde(skip_serializing_if = "Option::is_none")]
        usage: Option<AgentAsyncUsage>,
    },
    /// Task failed without a success payload.
    Failed {
        /// Agent identifier.
        agent_id: String,
        /// Asynchronous task identifier.
        async_id: String,
    },
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AgentAsyncWire {
    agent_id: Option<String>,
    async_id: Option<String>,
    status: Option<AgentAsyncStatus>,
    choices: Option<Vec<AgentAsyncChoice>>,
    usage: Option<AgentAsyncUsage>,
}

impl AgentAsyncResult {
    /// Return the task status.
    pub const fn status(&self) -> AgentAsyncStatus {
        match self {
            Self::Pending { .. } => AgentAsyncStatus::Pending,
            Self::Success { .. } => AgentAsyncStatus::Success,
            Self::Failed { .. } => AgentAsyncStatus::Failed,
        }
    }

    /// Validate invariants even for a value constructed in local code.
    pub fn validate(&self) -> ZaiResult<()> {
        match self {
            Self::Pending { agent_id, async_id } | Self::Failed { agent_id, async_id } => {
                if agent_id.trim().is_empty() || async_id.trim().is_empty() {
                    return Err(invalid_response(
                        "async result requires non-blank agent_id and async_id",
                    ));
                }
            },
            Self::Success {
                agent_id,
                async_id,
                choices,
                ..
            } => {
                if agent_id.trim().is_empty() || async_id.trim().is_empty() || choices.is_empty() {
                    return Err(invalid_response(
                        "successful async result requires ids and non-empty choices",
                    ));
                }
            },
        }
        Ok(())
    }
}

impl<'de> Deserialize<'de> for AgentAsyncResult {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;
        let object = value
            .as_object()
            .ok_or_else(|| D::Error::custom("agent async result must be a JSON object"))?;
        let has_result_fields = object.contains_key("choices") || object.contains_key("usage");
        let wire: AgentAsyncWire =
            serde_json::from_value(value).map_err(|error| D::Error::custom(error.to_string()))?;
        let status = wire
            .status
            .ok_or_else(|| D::Error::custom("agent async result omitted status"))?;
        let agent_id = required_text(wire.agent_id, "agent_id")
            .map_err(|error| D::Error::custom(error.to_string()))?;
        let async_id = required_text(wire.async_id, "async_id")
            .map_err(|error| D::Error::custom(error.to_string()))?;

        match status {
            AgentAsyncStatus::Pending | AgentAsyncStatus::Failed if has_result_fields => Err(
                D::Error::custom("non-success agent async result contained success-only fields"),
            ),
            AgentAsyncStatus::Pending => Ok(Self::Pending { agent_id, async_id }),
            AgentAsyncStatus::Failed => Ok(Self::Failed { agent_id, async_id }),
            AgentAsyncStatus::Success => {
                let choices = wire.choices.ok_or_else(|| {
                    D::Error::custom("successful agent async result omitted choices")
                })?;
                if choices.is_empty() {
                    return Err(D::Error::custom(
                        "successful agent async result contained empty choices",
                    ));
                }
                Ok(Self::Success {
                    agent_id,
                    async_id,
                    choices,
                    usage: wire.usage,
                })
            },
        }
    }
}

/// One slide-conversation response content part.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentConversationContentPart {
    /// Provider content type (`file_url` or `image_url`).
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub type_: Option<String>,
    /// Chinese description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag_cn: Option<String>,
    /// English description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag_en: Option<String>,
    /// Downloadable file URL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_url: Option<String>,
    /// Downloadable image URL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_url: Option<String>,
}

/// One message in a slide-conversation choice.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentConversationMessage {
    /// Response role.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    /// Generated content parts.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<Vec<AgentConversationContentPart>>,
}

/// One slide-conversation response choice.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentConversationChoice {
    /// Frozen schema uses singular `message` for this message array.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<Vec<AgentConversationMessage>>,
}

/// Embedded failure detail returned by the conversation schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentErrorDetail {
    /// Service error code.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    /// Human-readable failure message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl AgentErrorDetail {
    fn has_documented_value(&self) -> bool {
        self.code.is_some() || self.message.is_some()
    }
}

/// Successful slide-conversation response.
#[derive(Debug, Clone, Serialize)]
pub struct AgentConversationSuccess {
    /// Conversation identifier.
    pub conversation_id: String,
    /// Agent identifier.
    pub agent_id: String,
    /// Non-empty response choices.
    pub choices: Vec<AgentConversationChoice>,
}

/// Failed slide-conversation response.
#[derive(Debug, Clone, Serialize)]
pub struct AgentConversationFailure {
    /// Conversation identifier, when returned.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conversation_id: Option<String>,
    /// Agent identifier, when returned.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_id: Option<String>,
    /// Typed error detail.
    pub error: AgentErrorDetail,
}

/// Closed success/failure result from the slide-conversation endpoint.
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum AgentConversationResponse {
    /// Successful conversation continuation.
    Success(AgentConversationSuccess),
    /// Schema-defined embedded failure.
    Failed(AgentConversationFailure),
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AgentConversationWire {
    conversation_id: Option<String>,
    agent_id: Option<String>,
    choices: Option<Vec<AgentConversationChoice>>,
    error: Option<AgentErrorDetail>,
}

impl AgentConversationResponse {
    /// Validate invariants even for a value constructed in local code.
    pub fn validate(&self) -> ZaiResult<()> {
        match self {
            Self::Success(response) => {
                if response.conversation_id.trim().is_empty()
                    || response.agent_id.trim().is_empty()
                    || response.choices.is_empty()
                {
                    return Err(invalid_response(
                        "conversation success requires ids and non-empty choices",
                    ));
                }
            },
            Self::Failed(response) if !response.error.has_documented_value() => {
                return Err(invalid_response(
                    "conversation failure requires a documented error field",
                ));
            },
            Self::Failed(_) => {},
        }
        Ok(())
    }
}

impl<'de> Deserialize<'de> for AgentConversationResponse {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;
        let object = value
            .as_object()
            .ok_or_else(|| D::Error::custom("agent conversation response must be an object"))?;
        let has_error = object.contains_key("error");
        let has_choices = object.contains_key("choices");
        if has_error && has_choices {
            return Err(D::Error::custom(
                "agent conversation response mixed success and failure fields",
            ));
        }

        let wire: AgentConversationWire =
            serde_json::from_value(value).map_err(|error| D::Error::custom(error.to_string()))?;
        if has_error {
            let error = wire.error.ok_or_else(|| {
                D::Error::custom("agent conversation failure contained a null error")
            })?;
            if !error.has_documented_value() {
                return Err(D::Error::custom(
                    "agent conversation failure contained an empty error",
                ));
            }
            return Ok(Self::Failed(AgentConversationFailure {
                conversation_id: wire.conversation_id,
                agent_id: wire.agent_id,
                error,
            }));
        }

        let choices = wire
            .choices
            .ok_or_else(|| D::Error::custom("agent conversation success omitted choices"))?;
        if choices.is_empty() {
            return Err(D::Error::custom(
                "agent conversation success contained empty choices",
            ));
        }
        Ok(Self::Success(AgentConversationSuccess {
            conversation_id: required_text(wire.conversation_id, "conversation_id")
                .map_err(|error| D::Error::custom(error.to_string()))?,
            agent_id: required_text(wire.agent_id, "agent_id")
                .map_err(|error| D::Error::custom(error.to_string()))?,
            choices,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invoke_completed_uses_messages_array_without_a_status_tag() {
        let response: AgentInvokeResponse = serde_json::from_value(serde_json::json!({
            "id": "invoke-1",
            "agent_id": "general_translation",
            "conversation_id": "conversation-1",
            "choices": [{
                "index": 0,
                "messages": [{"role": "assistant", "content": "done"}],
                "finish_reason": "stop"
            }],
            "usage": {"prompt_tokens": 1, "completion_tokens": 2, "total_tokens": 3}
        }))
        .unwrap();

        let AgentInvokeResponse::Completed(completed) = response else {
            panic!("expected completed response");
        };
        assert_eq!(completed.id, "invoke-1");
        assert_eq!(completed.choices.len(), 1);
        assert_eq!(completed.choices[0].messages.as_ref().unwrap().len(), 1);
    }

    #[test]
    fn invoke_pending_has_no_synthetic_status_field() {
        let response: AgentInvokeResponse = serde_json::from_value(serde_json::json!({
            "agent_id": "general_translation",
            "async_id": "task-1"
        }))
        .unwrap();
        assert!(matches!(response, AgentInvokeResponse::Pending(_)));
    }

    #[test]
    fn invoke_rejects_empty_malformed_and_contradictory_shapes() {
        for value in [
            serde_json::json!({}),
            serde_json::json!({"status": "completed", "id": "i"}),
            serde_json::json!({"id": "i", "agent_id": "a", "choices": []}),
            serde_json::json!({
                "id": "i", "agent_id": "a", "async_id": "t", "choices": []
            }),
            serde_json::json!({
                "agent_id": "a", "async_id": "t", "choices": null
            }),
        ] {
            assert!(serde_json::from_value::<AgentInvokeResponse>(value).is_err());
        }
    }

    #[test]
    fn invoke_response_decodes_each_closed_content_form() {
        let response: AgentInvokeResponse = serde_json::from_value(serde_json::json!({
            "id": "invoke-1",
            "agent_id": "ai_drawing_agent",
            "choices": [{
                "messages": [
                    {"content": {"type": "image_url", "image_url": "https://example.test/one.png"}},
                    {"content": [
                        {"type": "text", "text": "caption"},
                        {"type": "image_url", "image_url": "https://example.test/two.png"}
                    ]}
                ]
            }]
        }))
        .unwrap();
        assert!(matches!(response, AgentInvokeResponse::Completed(_)));
    }

    #[test]
    fn async_result_decodes_all_statuses_and_exact_nested_messages() {
        let success: AgentAsyncResult = serde_json::from_value(serde_json::json!({
            "agent_id": "agent-1",
            "async_id": "task-1",
            "status": "success",
            "choices": [{
                "messages": [{
                    "role": "assistant",
                    "content": [{
                        "type": "file_url",
                        "file_url": "https://example.test/result.pdf",
                        "tag_cn": "结果",
                        "tag_en": "result"
                    }]
                }]
            }],
            "usage": {"total_tokens": 42}
        }))
        .unwrap();
        assert_eq!(success.status(), AgentAsyncStatus::Success);

        let pending: AgentAsyncResult = serde_json::from_value(serde_json::json!({
            "agent_id": "agent-1", "async_id": "task-1", "status": "pending"
        }))
        .unwrap();
        assert_eq!(pending.status(), AgentAsyncStatus::Pending);

        let failed: AgentAsyncResult = serde_json::from_value(serde_json::json!({
            "agent_id": "agent-1", "async_id": "task-1", "status": "failed"
        }))
        .unwrap();
        assert_eq!(failed.status(), AgentAsyncStatus::Failed);
    }

    #[test]
    fn async_result_rejects_empty_or_status_contradictions() {
        for value in [
            serde_json::json!({}),
            serde_json::json!({"agent_id": "a", "async_id": "t"}),
            serde_json::json!({
                "agent_id": "a", "async_id": "t", "status": "success", "choices": []
            }),
            serde_json::json!({
                "agent_id": "a", "async_id": "t", "status": "pending", "choices": null
            }),
        ] {
            assert!(serde_json::from_value::<AgentAsyncResult>(value).is_err());
        }
    }

    #[test]
    fn conversation_uses_singular_message_array_and_typed_error() {
        let success: AgentConversationResponse = serde_json::from_value(serde_json::json!({
            "conversation_id": "conversation-1",
            "agent_id": "slides_glm_agent",
            "choices": [{
                "message": [{
                    "role": "assistant",
                    "content": [{
                        "type": "file_url",
                        "file_url": "https://example.test/slides.pptx",
                        "tag_cn": "演示文稿",
                        "tag_en": "slides"
                    }]
                }]
            }]
        }))
        .unwrap();
        assert!(matches!(success, AgentConversationResponse::Success(_)));

        let failed: AgentConversationResponse = serde_json::from_value(serde_json::json!({
            "conversation_id": "conversation-1",
            "agent_id": "slides_glm_agent",
            "error": {"code": "invalid_slide", "message": "could not render"}
        }))
        .unwrap();
        assert!(matches!(failed, AgentConversationResponse::Failed(_)));
    }

    #[test]
    fn conversation_rejects_empty_wrong_and_contradictory_shapes() {
        for value in [
            serde_json::json!({}),
            serde_json::json!({
                "conversation_id": "c", "agent_id": "a", "choices": []
            }),
            serde_json::json!({
                "conversation_id": "c", "agent_id": "a",
                "choices": [{"messages": []}]
            }),
            serde_json::json!({"error": {}}),
            serde_json::json!({
                "conversation_id": "c", "agent_id": "a", "choices": [{}],
                "error": {"message": "failed"}
            }),
        ] {
            assert!(serde_json::from_value::<AgentConversationResponse>(value).is_err());
        }
    }
}
