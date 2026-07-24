//! # Chat Response Types
//!
//! Defines the standard response structures returned by chat-completion
//! endpoints, including choices, usage statistics, and task-status tracking
//! for async operations.
//!
//! Notes:
//! - All fields are optional unless documented otherwise; servers may omit
//!   fields or return null.
//! - Some IDs may be numbers on the wire; we normalize them to `String` via
//!   custom deserializers.
//! - In non-stream responses, `choices` typically has length 1 unless the API
//!   supports multi-candidate responses.
use serde::{Deserialize, Deserializer, Serialize, de::Error as _};
use validator::Validate;

/// Successful business response (HTTP 200, application/json).
/// Notes:
/// - `choices` is often a single element in non-stream mode unless explicitly
///   requested otherwise.
/// - `id`/`request_id` are normalized to `String` even if the server returns
///   numbers.
/// - `usage` is typically present only after completion (not during streaming).

#[derive(Clone, Serialize, Validate)]
pub struct ChatCompletionResponse {
    /// Task ID
    #[serde(
        skip_serializing_if = "Option::is_none",
        deserialize_with = "super::serde_helpers::optional_string_from_number_or_string"
    )]
    pub id: Option<String>,

    /// Request ID
    #[serde(
        skip_serializing_if = "Option::is_none",
        deserialize_with = "super::serde_helpers::optional_string_from_number_or_string"
    )]
    pub request_id: Option<String>,

    /// Request created time, Unix timestamp (seconds)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created: Option<u64>,

    /// Model name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    /// Model response list
    #[serde(skip_serializing_if = "Option::is_none")]
    pub choices: Option<Vec<Choice>>,

    /// Token usage statistics at the end of the call
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<Usage>,

    /// Video generation results
    #[serde(skip_serializing_if = "Option::is_none")]
    pub video_result: Option<Vec<VideoResultItem>>,

    /// Information related to web search, returned when using
    /// WebSearchToolSchema
    #[serde(skip_serializing_if = "Option::is_none")]
    pub web_search: Option<Vec<WebSearchInfo>>,

    /// Content safety related information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_filter: Option<Vec<ContentFilterInfo>>,
    /// Processing status of the task: `PROCESSING`, `SUCCESS`, or `FAIL`.
    /// While processing, the final result needs
    /// to be retrieved via a subsequent query.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_status: Option<TaskStatus>,
}

#[derive(Deserialize)]
struct ChatCompletionResponseWire {
    #[serde(
        default,
        deserialize_with = "super::serde_helpers::optional_string_from_number_or_string"
    )]
    id: Option<String>,
    #[serde(
        default,
        deserialize_with = "super::serde_helpers::optional_string_from_number_or_string"
    )]
    request_id: Option<String>,
    created: Option<u64>,
    model: Option<String>,
    choices: Option<Vec<Choice>>,
    usage: Option<Usage>,
    video_result: Option<Vec<VideoResultItem>>,
    web_search: Option<Vec<WebSearchInfo>>,
    content_filter: Option<Vec<ContentFilterInfo>>,
    task_status: Option<TaskStatus>,
}

impl<'de> Deserialize<'de> for ChatCompletionResponse {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = ChatCompletionResponseWire::deserialize(deserializer)?;
        let response = Self {
            id: wire.id,
            request_id: wire.request_id,
            created: wire.created,
            model: wire.model,
            choices: wire.choices,
            usage: wire.usage,
            video_result: wire.video_result,
            web_search: wire.web_search,
            content_filter: wire.content_filter,
            task_status: wire.task_status,
        };
        if response.id.is_none()
            && response.request_id.is_none()
            && response.created.is_none()
            && response.model.is_none()
            && response.choices.is_none()
            && response.usage.is_none()
            && response.video_result.is_none()
            && response.web_search.is_none()
            && response.content_filter.is_none()
            && response.task_status.is_none()
        {
            return Err(D::Error::custom(
                "chat completion response contained no documented fields",
            ));
        }
        Ok(response)
    }
}

impl std::fmt::Debug for ChatCompletionResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match serde_json::to_string_pretty(self) {
            Ok(s) => f.write_str(&s),
            Err(_) => f.debug_struct("ChatCompletionResponse").finish(),
        }
    }
}
/// Task processing status.
/// Values correspond to upstream payload strings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum TaskStatus {
    /// Task is still running; poll again to retrieve the final result.
    #[serde(rename = "PROCESSING", alias = "processing")]
    Processing,
    /// Task completed successfully.
    #[serde(rename = "SUCCESS", alias = "success")]
    Success,
    /// Task failed.
    #[serde(rename = "FAIL", alias = "fail")]
    Fail,
    /// An unrecognized status returned by a newer API version. The catch-all
    /// (`#[serde(other)]`) keeps a single unknown value from failing the whole
    /// response deserialization; callers should treat it as not-yet-complete
    /// (keep polling) or surface it, rather than aborting.
    #[serde(rename = "UNKNOWN", other)]
    Unknown,
}
impl TaskStatus {
    /// Return the canonical upstream string for this status
    /// (`"PROCESSING"` / `"SUCCESS"` / `"FAIL"` / `"UNKNOWN"`).
    pub fn as_str(&self) -> &'static str {
        match self {
            TaskStatus::Processing => "PROCESSING",
            TaskStatus::Success => "SUCCESS",
            TaskStatus::Fail => "FAIL",
            TaskStatus::Unknown => "UNKNOWN",
        }
    }
}

impl std::fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod task_status_tests {
    use super::TaskStatus;

    #[test]
    fn unknown_status_serializes_consistently_with_display() {
        assert_eq!(
            serde_json::to_string(&TaskStatus::Unknown).unwrap(),
            r#""UNKNOWN""#
        );
        assert_eq!(TaskStatus::Unknown.to_string(), "UNKNOWN");
    }
}

/// One choice item in the response.
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct Choice {
    /// Index of this result
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<i32>,

    /// Message content
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<Message>,

    /// Why generation finished

    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
}

/// Notes:
/// - Depending on the model/mode, only one of `content`, `audio`, or
///   `tool_calls` may be set.
/// - Prefer `content` for final text; `reasoning_content` may contain internal
///   traces (when available).
///
/// Assistant message payload
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct Message {
    /// Role of the message, defaults to "assistant"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,

    /// Current dialog content.
    /// If function/tool calling is used, this may be null; otherwise contains
    /// the inference result. For some models, content may include thinking
    /// traces within `<think>` tags, with final output outside.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<MessageContent>,

    /// Reasoning chain content (only for specific models)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_content: Option<String>,

    /// Audio payload for voice models (glm-4-voice)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio: Option<AudioContent>,

    /// Generated tool/function calls
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCallMessage>>,
}

/// Assistant response content in either plain-text or multimodal-parts form.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageContent {
    /// Plain assistant text.
    Text(String),
    /// Multimodal response parts.
    Parts(Vec<MessageContentPart>),
}

impl MessageContent {
    /// Borrow plain text when this is the text form.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::Text(text) => Some(text),
            Self::Parts(_) => None,
        }
    }

    /// Borrow multimodal parts when this is the parts form.
    pub fn as_parts(&self) -> Option<&[MessageContentPart]> {
        match self {
            Self::Text(_) => None,
            Self::Parts(parts) => Some(parts),
        }
    }
}

/// One item in a multimodal assistant response.
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct MessageContentPart {
    /// Part kind. The current response schema only defines `text`.
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub type_: Option<MessageContentPartType>,
    /// Text carried by this part.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

/// Supported multimodal assistant response part kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageContentPartType {
    /// Text response part.
    Text,
}

/// Tool/function call description inside message
/// Notes:
/// - When `function` is present, `type` is typically "function"; `mcp` is used
///   for MCP calls.
/// - `id` is normalized to `String` (server may return numbers).

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct ToolCallMessage {
    /// Unique id of this tool/function call (server may return numbers).
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "super::serde_helpers::optional_string_from_number_or_string"
    )]
    pub id: Option<String>,
    /// Tool call type — typically `"function"` for function calls.
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub type_: Option<String>,
    /// Function-call payload (name + arguments) when `type` is `"function"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function: Option<ToolFunction>,
    /// MCP tool call payload (when type indicates MCP)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mcp: Option<MCPMessage>,
}

/// Function-call payload inside a [`ToolCallMessage`].
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct ToolFunction {
    /// Name of the function/tool to invoke.
    pub name: String,
    /// JSON-encoded arguments to pass to the function.
    pub arguments: String,
}

/// MCP tool call payload
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct MCPMessage {
    /// Unique id of this MCP tool call
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "super::serde_helpers::optional_string_from_number_or_string"
    )]
    pub id: Option<String>,
    /// Tool call type: mcp_list_tools, mcp_call
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub type_: Option<MCPCallType>,
    /// MCP server label
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_label: Option<String>,
    /// Error message if any
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,

    /// Tool list when type = mcp_list_tools
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<MCPTool>>,

    /// Tool call arguments (JSON string) when type = mcp_call. A directly
    /// encoded JSON value is normalized to its compact string representation.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "super::serde_helpers::optional_json_string"
    )]
    pub arguments: Option<String>,
    /// Tool name when type = mcp_call
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Tool returned output when type = mcp_call
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<serde_json::Value>,
}

/// MCP tool call type — either a tool-list request or an actual tool
/// invocation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
#[serde(rename_all = "snake_case")]
pub enum MCPCallType {
    /// Request the server to list its available MCP tools.
    McpListTools,
    /// Invoke a specific MCP tool.
    McpCall,
    /// An unrecognized `type` returned by a newer API or a different MCP
    /// transport. Deserialized via `#[serde(other)]` so a novel value no longer
    /// fails the whole response.
    #[serde(other)]
    Unknown,
}

/// Tool descriptor reported by an MCP server.
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct MCPTool {
    /// Tool name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Tool description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Tool annotations
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<serde_json::Value>,
    /// Tool input schema
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_schema: Option<MCPInputSchema>,
}
/// JSON-schema-like input descriptor for an MCP tool.
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct MCPInputSchema {
    /// Fixed value 'object'
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub type_: Option<MCPInputType>,
    /// Parameter properties definition
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<serde_json::Value>,
    /// Required property list
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<Vec<String>>,
    /// Whether additional properties are allowed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_properties: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Input schema type for MCP tools.
/// Currently only `object` is observed; kept as an enum for forward
/// compatibility.
#[non_exhaustive]
#[serde(rename_all = "lowercase")]
pub enum MCPInputType {
    /// JSON-schema `object` type (the only value observed today).
    Object,
    /// An unrecognized schema `type` from a newer MCP version. Deserialized via
    /// `#[serde(other)]` so a novel value no longer fails the whole response.
    #[serde(other)]
    Unknown,
}

/// Audio content returned for voice models.
/// Notes:
/// - `data` is base64-encoded audio bytes (e.g., WAV/MP3) — decode before
///   saving/playing.
/// - `id` and `expires_at` are normalized to `String` and may be numeric on the
///   wire.

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct AudioContent {
    /// Audio content id, can be used for multi-turn inputs
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "super::serde_helpers::optional_string_from_number_or_string"
    )]
    pub id: Option<String>,
    /// Base64 encoded audio data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
    /// Expiration time for the audio content
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "super::serde_helpers::optional_string_from_number_or_string"
    )]
    pub expires_at: Option<String>,
}

/// Token usage statistics.
/// Notes:
/// - `total_tokens` ≈ `prompt_tokens` + `completion_tokens`.
/// - Some providers omit `usage` in streaming chunks; expect it mainly in the
///   final response.
/// - `prompt_tokens_details.cached_tokens` often indicates KV-cache hits or
///   reused tokens.

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct Usage {
    /// Number of tokens in the prompt.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_tokens: Option<u32>,
    /// Number of tokens in the completion.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completion_tokens: Option<u32>,
    /// Total tokens for this request (`prompt` + `completion`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_tokens: Option<u32>,
    /// Details for prompt tokens (e.g., cached tokens count)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_tokens_details: Option<PromptTokensDetails>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
/// Details for how prompt tokens were accounted.
/// Fields here are provider-specific and may expand in the future.
pub struct PromptTokensDetails {
    /// Number of tokens hit by cache
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cached_tokens: Option<u32>,
}

/// Web search item returned by the service.
/// Notes:
/// - `link` and media URLs may be temporary; consider downloading or caching if
///   needed.
/// - Fields are optional and may vary by search provider/source.

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct WebSearchInfo {
    /// Source website icon
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    /// Search result title
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Search result page link
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(url)]
    pub link: Option<String>,
    /// Media source name of the page
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media: Option<String>,
    /// Publish date on the website
    #[serde(skip_serializing_if = "Option::is_none")]
    pub publish_date: Option<String>,
    /// Quoted text content from the search result page
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    /// Corner mark sequence number
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refer: Option<String>,
}

/// Video generation result item.
/// Notes:
/// - URLs may be temporary; fetch/save promptly if you need persistence.
/// - Some providers deliver video asynchronously; this URL may point to a
///   job/result resource.

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct VideoResultItem {
    /// Video link
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(url)]
    pub url: Option<String>,
    /// Cover image link
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(url)]
    pub cover_image_url: Option<String>,
}

/// Content safety information item.
/// Notes:
/// - Use `role` + `level` to decide block/warn/allow strategies.
/// - Providers may add categories or additional fields in the future.

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct ContentFilterInfo {
    /// Stage where the safety check applies: assistant (model inference), user
    /// (user input), history (context)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,

    /// Severity level 0-3 (0 most severe, 3 minor)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(range(min = 0, max = 3))]
    pub level: Option<i32>,
}

// Accessors keep response fields encapsulated while preserving zero-copy reads.
impl ChatCompletionResponse {
    /// Task id (normalized to `&str`).
    pub fn id(&self) -> Option<&str> {
        self.id.as_deref()
    }
    /// Request id (normalized to `&str`).
    pub fn request_id(&self) -> Option<&str> {
        self.request_id.as_deref()
    }
    /// Unix timestamp (seconds) at which the request was created.
    pub fn created(&self) -> Option<u64> {
        self.created
    }
    /// Model name that produced the response.
    pub fn model(&self) -> Option<&str> {
        self.model.as_deref()
    }
    /// Generated choices (typically one in non-stream mode).
    pub fn choices(&self) -> Option<&[Choice]> {
        self.choices.as_deref()
    }
    /// Token usage statistics (mainly on the final response).
    pub fn usage(&self) -> Option<&Usage> {
        self.usage.as_ref()
    }
    /// Video generation result items, if any.
    pub fn video_result(&self) -> Option<&[VideoResultItem]> {
        self.video_result.as_deref()
    }
    /// Web-search citations, if `web_search` was used.
    pub fn web_search(&self) -> Option<&[WebSearchInfo]> {
        self.web_search.as_deref()
    }
    /// Content-safety filter results, if any.
    pub fn content_filter(&self) -> Option<&[ContentFilterInfo]> {
        self.content_filter.as_deref()
    }
    /// Async task status, if this is an async response.
    pub fn task_status(&self) -> Option<&TaskStatus> {
        self.task_status.as_ref()
    }
}

impl Choice {
    /// Index of this choice within the `choices` array.
    pub fn index(&self) -> Option<i32> {
        self.index
    }
    /// The assistant message payload, when returned.
    pub fn message(&self) -> Option<&Message> {
        self.message.as_ref()
    }
    /// Reason generation finished (e.g. `"stop"`, `"length"`).
    pub fn finish_reason(&self) -> Option<&str> {
        self.finish_reason.as_deref()
    }
}

impl Message {
    /// Role of the message (typically `"assistant"`).
    pub fn role(&self) -> Option<&str> {
        self.role.as_deref()
    }
    /// Dialog content (may include `<think>` traces for some models).
    pub fn content(&self) -> Option<&MessageContent> {
        self.content.as_ref()
    }
    /// Return the assistant text when `content` is a JSON string; `None`
    /// otherwise (absent, null, or the array-of-parts form). Convenience over
    /// [`content`](Self::content) for the common case where the model returns a
    /// plain string.
    pub fn content_str(&self) -> Option<&str> {
        self.content.as_ref().and_then(MessageContent::as_str)
    }
    /// Reasoning-chain content, when the model exposes it.
    pub fn reasoning_content(&self) -> Option<&str> {
        self.reasoning_content.as_deref()
    }
    /// Audio payload, for voice models.
    pub fn audio(&self) -> Option<&AudioContent> {
        self.audio.as_ref()
    }
    /// Tool/function calls the model wants the caller to execute.
    pub fn tool_calls(&self) -> Option<&[ToolCallMessage]> {
        self.tool_calls.as_deref()
    }
}

impl ToolCallMessage {
    /// Unique id of this tool/function call.
    pub fn id(&self) -> Option<&str> {
        self.id.as_deref()
    }
    /// Tool call type (typically `"function"`).
    pub fn type_(&self) -> Option<&str> {
        self.type_.as_deref()
    }
    /// Function-call payload (name + arguments).
    pub fn function(&self) -> Option<&ToolFunction> {
        self.function.as_ref()
    }
    /// MCP tool call payload, when applicable.
    pub fn mcp(&self) -> Option<&MCPMessage> {
        self.mcp.as_ref()
    }
}

impl ToolFunction {
    /// Name of the function/tool to invoke.
    pub fn name(&self) -> &str {
        &self.name
    }
    /// JSON-encoded arguments for the function call.
    pub fn arguments(&self) -> &str {
        &self.arguments
    }
}

impl MCPMessage {
    /// Unique id of this MCP tool call.
    pub fn id(&self) -> Option<&str> {
        self.id.as_deref()
    }
    /// MCP call type (`mcp_list_tools` / `mcp_call`).
    pub fn type_(&self) -> Option<&MCPCallType> {
        self.type_.as_ref()
    }
    /// MCP server label.
    pub fn server_label(&self) -> Option<&str> {
        self.server_label.as_deref()
    }
    /// Error message reported by the MCP server, if any.
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }
    /// Tools advertised by the server (when `type` is `mcp_list_tools`).
    pub fn tools(&self) -> Option<&[MCPTool]> {
        self.tools.as_deref()
    }
    /// JSON-encoded call arguments (when `type` is `mcp_call`).
    pub fn arguments(&self) -> Option<&str> {
        self.arguments.as_deref()
    }
    /// Tool name invoked (when `type` is `mcp_call`).
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }
    /// Raw tool output returned by the server (when `type` is `mcp_call`).
    pub fn output(&self) -> Option<&serde_json::Value> {
        self.output.as_ref()
    }
}

impl MCPTool {
    /// Tool name.
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }
    /// Human-readable tool description.
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }
    /// Tool annotations (provider-specific).
    pub fn annotations(&self) -> Option<&serde_json::Value> {
        self.annotations.as_ref()
    }
    /// Input schema describing the tool's parameters.
    pub fn input_schema(&self) -> Option<&MCPInputSchema> {
        self.input_schema.as_ref()
    }
}

impl MCPInputSchema {
    /// Schema type (currently always `object`).
    pub fn type_(&self) -> Option<&MCPInputType> {
        self.type_.as_ref()
    }
    /// Property definitions of the schema.
    pub fn properties(&self) -> Option<&serde_json::Value> {
        self.properties.as_ref()
    }
    /// List of required property names.
    pub fn required(&self) -> Option<&[String]> {
        self.required.as_deref()
    }
    /// Whether properties beyond `properties` are permitted.
    pub fn additional_properties(&self) -> Option<bool> {
        self.additional_properties
    }
}

impl AudioContent {
    /// Audio content id (usable for multi-turn inputs).
    pub fn id(&self) -> Option<&str> {
        self.id.as_deref()
    }
    /// Base64-encoded audio data.
    pub fn data(&self) -> Option<&str> {
        self.data.as_deref()
    }
    /// Expiration timestamp of the audio content.
    pub fn expires_at(&self) -> Option<&str> {
        self.expires_at.as_deref()
    }
}

impl Usage {
    /// Number of prompt tokens.
    pub fn prompt_tokens(&self) -> Option<u32> {
        self.prompt_tokens
    }
    /// Number of completion tokens.
    pub fn completion_tokens(&self) -> Option<u32> {
        self.completion_tokens
    }
    /// Total tokens for this request.
    pub fn total_tokens(&self) -> Option<u32> {
        self.total_tokens
    }
    /// Breakdown of prompt-token accounting.
    pub fn prompt_tokens_details(&self) -> Option<&PromptTokensDetails> {
        self.prompt_tokens_details.as_ref()
    }
}

impl PromptTokensDetails {
    /// Number of prompt tokens served from cache.
    pub fn cached_tokens(&self) -> Option<u32> {
        self.cached_tokens
    }
}

impl WebSearchInfo {
    /// Source website icon URL.
    pub fn icon(&self) -> Option<&str> {
        self.icon.as_deref()
    }
    /// Search-result title.
    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }
    /// Search-result page URL.
    pub fn link(&self) -> Option<&str> {
        self.link.as_deref()
    }
    /// Media/source name of the page.
    pub fn media(&self) -> Option<&str> {
        self.media.as_deref()
    }
    /// Publish date of the page.
    pub fn publish_date(&self) -> Option<&str> {
        self.publish_date.as_deref()
    }
    /// Quoted snippet from the result page.
    pub fn content(&self) -> Option<&str> {
        self.content.as_deref()
    }
    /// Reference marker (corner number) for the citation.
    pub fn refer(&self) -> Option<&str> {
        self.refer.as_deref()
    }
}

impl VideoResultItem {
    /// Generated video URL.
    pub fn url(&self) -> Option<&str> {
        self.url.as_deref()
    }
    /// Cover-image URL for the video.
    pub fn cover_image_url(&self) -> Option<&str> {
        self.cover_image_url.as_deref()
    }
}

impl ContentFilterInfo {
    /// Safety-check stage (`assistant`, `user`, or `history`).
    pub fn role(&self) -> Option<&str> {
        self.role.as_deref()
    }
    /// Severity level (`0` most severe … `3` minor).
    pub fn level(&self) -> Option<i32> {
        self.level
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_function_requires_the_documented_string_fields() {
        let tf: ToolFunction =
            serde_json::from_str(r#"{"name":"f","arguments":"{\"a\":1}"}"#).unwrap();
        assert_eq!(tf.name, "f");
        assert_eq!(tf.arguments, r#"{"a":1}"#);
        assert!(serde_json::from_str::<ToolFunction>(r#"{"arguments":"{}"}"#).is_err());
        assert!(serde_json::from_str::<ToolFunction>(r#"{"name":"f"}"#).is_err());
        assert!(serde_json::from_str::<ToolFunction>(r#"{"name":"f","arguments":{}}"#).is_err());
    }

    #[test]
    fn mcp_message_arguments_lenient() {
        let m: MCPMessage = serde_json::from_str(r#"{"id":"x","arguments":{}}"#).unwrap();
        assert_eq!(m.arguments.as_deref(), Some("{}"));
        let m: MCPMessage = serde_json::from_str(r#"{"id":"x","arguments":null}"#).unwrap();
        assert!(m.arguments.is_none());
    }

    #[test]
    fn optional_custom_deserialized_fields_may_be_omitted() {
        let tool_call: ToolCallMessage = serde_json::from_str("{}").unwrap();
        assert!(tool_call.id.is_none());
        let mcp: MCPMessage = serde_json::from_str("{}").unwrap();
        assert!(mcp.id.is_none());
        assert!(mcp.arguments.is_none());
        let audio: AudioContent = serde_json::from_str("{}").unwrap();
        assert!(audio.id.is_none());
        assert!(audio.expires_at.is_none());
    }

    #[test]
    fn completion_response_rejects_empty_success_bodies() {
        assert!(serde_json::from_str::<ChatCompletionResponse>("{}").is_err());
        assert!(serde_json::from_str::<ChatCompletionResponse>(r#"{"id":null}"#).is_err());
        assert!(serde_json::from_str::<ChatCompletionResponse>(r#"{"id":"task-1"}"#).is_ok());
        assert!(serde_json::from_str::<ChatCompletionResponse>(r#"{"choices":[]}"#).is_ok());
    }

    #[test]
    fn choice_fields_follow_their_optional_openapi_shape() {
        let choice: Choice = serde_json::from_str("{}").unwrap();
        assert_eq!(choice.index(), None);
        assert!(choice.message().is_none());
    }

    #[test]
    fn assistant_content_accepts_only_the_documented_union() {
        let text: Message = serde_json::from_value(serde_json::json!({
            "content": "hello"
        }))
        .unwrap();
        assert_eq!(text.content_str(), Some("hello"));

        let parts: Message = serde_json::from_value(serde_json::json!({
            "content": [{"type": "text", "text": "hello"}]
        }))
        .unwrap();
        assert_eq!(
            parts
                .content()
                .and_then(MessageContent::as_parts)
                .and_then(|parts| parts.first())
                .and_then(|part| part.text.as_deref()),
            Some("hello")
        );

        assert!(serde_json::from_value::<Message>(serde_json::json!({"content": 42})).is_err());
        assert!(serde_json::from_value::<Message>(serde_json::json!({"content": {}})).is_err());
    }

    #[test]
    fn mcp_call_type_known_values_round_trip() {
        let m: MCPMessage = serde_json::from_str(r#"{"id":"x","type":"mcp_call"}"#).unwrap();
        assert!(matches!(m.type_, Some(MCPCallType::McpCall)));
        let m: MCPMessage = serde_json::from_str(r#"{"id":"x","type":"mcp_list_tools"}"#).unwrap();
        assert!(matches!(m.type_, Some(MCPCallType::McpListTools)));
    }

    #[test]
    fn mcp_call_type_unknown_value_falls_back_to_unknown() {
        // A novel `type` from a newer API/MCP transport must not fail the whole
        // response — it maps to the `#[serde(other)]` catch-all.
        let m: MCPMessage =
            serde_json::from_str(r#"{"id":"x","type":"mcp_future_transport"}"#).unwrap();
        assert!(matches!(m.type_, Some(MCPCallType::Unknown)));
    }

    #[test]
    fn mcp_input_type_unknown_value_falls_back_to_unknown() {
        let s: MCPInputSchema = serde_json::from_str(r#"{"type":"array"}"#).unwrap();
        assert!(matches!(s.type_, Some(MCPInputType::Unknown)));
        let s: MCPInputSchema = serde_json::from_str(r#"{"type":"object"}"#).unwrap();
        assert!(matches!(s.type_, Some(MCPInputType::Object)));
    }
}
