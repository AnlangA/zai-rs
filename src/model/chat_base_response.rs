//! Base response types for chat API models.
//!
//! This module defines the standard response structures for 200 application/json
//! responses from the service.

use serde::Deserialize;
use validator::Validate;


/// Successful business response (HTTP 200, application/json).
#[derive(Debug, Clone, Deserialize, Validate)]
pub struct ChatCompletionResponse {
    /// Task ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// Request ID
    #[serde(skip_serializing_if = "Option::is_none")]
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

    /// Information related to web search, returned when using WebSearchToolSchema
    #[serde(skip_serializing_if = "Option::is_none")]
    pub web_search: Option<Vec<WebSearchInfo>>,

    /// Content safety related information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_filter: Option<Vec<ContentFilterInfo>>,
}


/// One choice item in the response.
#[derive(Debug, Clone, Deserialize, Validate)]
pub struct Choice {
    /// Index of this result
    pub index: i32,

    /// Message content
    pub message: Message,

    /// Why generation finished
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
}

/// Assistant message payload
#[derive(Debug, Clone, Deserialize, Validate)]
pub struct Message {
    /// Role of the message, defaults to "assistant"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,

    /// Current dialog content.
    /// If function/tool calling is used, this may be null; otherwise contains the inference result.
    /// For some models, content may include thinking traces within <think> tags, with final output outside.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<serde_json::Value>,

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

/// Tool/function call description inside message
#[derive(Debug, Clone, Deserialize, Validate)]
pub struct ToolCallMessage {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub type_: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function: Option<ToolFunction>,
    /// MCP tool call payload (when type indicates MCP)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mcp: Option<MCPMessage>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct ToolFunction {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<String>,
}

/// MCP tool call payload
#[derive(Debug, Clone, Deserialize, Validate)]
pub struct MCPMessage {
    /// Unique id of this MCP tool call
    #[serde(skip_serializing_if = "Option::is_none")]
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

    /// Tool call arguments (JSON string) when type = mcp_call
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<String>,
    /// Tool name when type = mcp_call
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Tool returned output when type = mcp_call
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MCPCallType {
    McpListTools,
    McpCall,
}

#[derive(Debug, Clone, Deserialize, Validate)]
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
#[derive(Debug, Clone, Deserialize, Validate)]
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

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MCPInputType {
    Object,
}


/// Audio content returned for voice models.
#[derive(Debug, Clone, Deserialize, Validate)]
pub struct AudioContent {
    /// Audio content id, can be used for multi-turn inputs
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Base64 encoded audio data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
    /// Expiration time for the audio content
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
}



/// Token usage statistics.
#[derive(Debug, Clone, Deserialize, Validate)]
pub struct Usage {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completion_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_tokens: Option<u32>,
    /// Details for prompt tokens (e.g., cached tokens count)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_tokens_details: Option<PromptTokensDetails>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct PromptTokensDetails {
    /// Number of tokens hit by cache
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cached_tokens: Option<u32>,
}

/// Web search item returned by the service.
#[derive(Debug, Clone, Deserialize, Validate)]
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
#[derive(Debug, Clone, Deserialize, Validate)]
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
#[derive(Debug, Clone, Deserialize, Validate)]
pub struct ContentFilterInfo {
    /// Stage where the safety check applies: assistant (model inference), user (user input), history (context)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,

    /// Severity level 0-3 (0 most severe, 3 minor)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(range(min = 0, max = 3))]
    pub level: Option<i32>,
}



// Getter implementations
impl ChatCompletionResponse {
    pub fn id(&self) -> Option<&str> { self.id.as_deref() }
    pub fn request_id(&self) -> Option<&str> { self.request_id.as_deref() }
    pub fn created(&self) -> Option<u64> { self.created }
    pub fn model(&self) -> Option<&str> { self.model.as_deref() }
    pub fn choices(&self) -> Option<&[Choice]> { self.choices.as_deref() }
    pub fn usage(&self) -> Option<&Usage> { self.usage.as_ref() }
    pub fn video_result(&self) -> Option<&[VideoResultItem]> { self.video_result.as_deref() }
    pub fn web_search(&self) -> Option<&[WebSearchInfo]> { self.web_search.as_deref() }
    pub fn content_filter(&self) -> Option<&[ContentFilterInfo]> { self.content_filter.as_deref() }
}

impl Choice {
    pub fn index(&self) -> i32 { self.index }
    pub fn message(&self) -> &Message { &self.message }
    pub fn finish_reason(&self) -> Option<&str> { self.finish_reason.as_deref() }
}

impl Message {
    pub fn role(&self) -> Option<&str> { self.role.as_deref() }
    pub fn content(&self) -> Option<&serde_json::Value> { self.content.as_ref() }
    pub fn reasoning_content(&self) -> Option<&str> { self.reasoning_content.as_deref() }
    pub fn audio(&self) -> Option<&AudioContent> { self.audio.as_ref() }
    pub fn tool_calls(&self) -> Option<&[ToolCallMessage]> { self.tool_calls.as_deref() }
}

impl ToolCallMessage {
    pub fn id(&self) -> Option<&str> { self.id.as_deref() }
    pub fn type_(&self) -> Option<&str> { self.type_.as_deref() }
    pub fn function(&self) -> Option<&ToolFunction> { self.function.as_ref() }
    pub fn mcp(&self) -> Option<&MCPMessage> { self.mcp.as_ref() }
}

impl ToolFunction {
    pub fn name(&self) -> Option<&str> { self.name.as_deref() }
    pub fn arguments(&self) -> Option<&str> { self.arguments.as_deref() }
}

impl MCPMessage {
    pub fn id(&self) -> Option<&str> { self.id.as_deref() }
    pub fn type_(&self) -> Option<&MCPCallType> { self.type_.as_ref() }
    pub fn server_label(&self) -> Option<&str> { self.server_label.as_deref() }
    pub fn error(&self) -> Option<&str> { self.error.as_deref() }
    pub fn tools(&self) -> Option<&[MCPTool]> { self.tools.as_deref() }
    pub fn arguments(&self) -> Option<&str> { self.arguments.as_deref() }
    pub fn name(&self) -> Option<&str> { self.name.as_deref() }
    pub fn output(&self) -> Option<&serde_json::Value> { self.output.as_ref() }
}

impl MCPTool {
    pub fn name(&self) -> Option<&str> { self.name.as_deref() }
    pub fn description(&self) -> Option<&str> { self.description.as_deref() }
    pub fn annotations(&self) -> Option<&serde_json::Value> { self.annotations.as_ref() }
    pub fn input_schema(&self) -> Option<&MCPInputSchema> { self.input_schema.as_ref() }
}

impl MCPInputSchema {
    pub fn type_(&self) -> Option<&MCPInputType> { self.type_.as_ref() }
    pub fn properties(&self) -> Option<&serde_json::Value> { self.properties.as_ref() }
    pub fn required(&self) -> Option<&[String]> { self.required.as_deref() }
    pub fn additional_properties(&self) -> Option<bool> { self.additional_properties }
}

impl AudioContent {
    pub fn id(&self) -> Option<&str> { self.id.as_deref() }
    pub fn data(&self) -> Option<&str> { self.data.as_deref() }
    pub fn expires_at(&self) -> Option<&str> { self.expires_at.as_deref() }
}

impl Usage {
    pub fn prompt_tokens(&self) -> Option<u32> { self.prompt_tokens }
    pub fn completion_tokens(&self) -> Option<u32> { self.completion_tokens }
    pub fn total_tokens(&self) -> Option<u32> { self.total_tokens }
    pub fn prompt_tokens_details(&self) -> Option<&PromptTokensDetails> { self.prompt_tokens_details.as_ref() }
}

impl PromptTokensDetails {
    pub fn cached_tokens(&self) -> Option<u32> { self.cached_tokens }
}

impl WebSearchInfo {
    pub fn icon(&self) -> Option<&str> { self.icon.as_deref() }
    pub fn title(&self) -> Option<&str> { self.title.as_deref() }
    pub fn link(&self) -> Option<&str> { self.link.as_deref() }
    pub fn media(&self) -> Option<&str> { self.media.as_deref() }
    pub fn publish_date(&self) -> Option<&str> { self.publish_date.as_deref() }
    pub fn content(&self) -> Option<&str> { self.content.as_deref() }
    pub fn refer(&self) -> Option<&str> { self.refer.as_deref() }
}

impl VideoResultItem {
    pub fn url(&self) -> Option<&str> { self.url.as_deref() }
    pub fn cover_image_url(&self) -> Option<&str> { self.cover_image_url.as_deref() }
}

impl ContentFilterInfo {
    pub fn role(&self) -> Option<&str> { self.role.as_deref() }
    pub fn level(&self) -> Option<i32> { self.level }
}
