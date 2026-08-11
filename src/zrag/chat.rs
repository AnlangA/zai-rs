use std::{
    pin::Pin,
    task::{Context, Poll},
};

use futures_util::{Stream, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::{
    ZaiError, ZaiResult,
    client::{ZaiClient, error::codes},
    serde_helpers::UniqueJsonValue,
};

const SESSION_HEADER: &str = "X-Session-Id";

/// Content of one user message sent to the ZRAG agent.
#[derive(Clone, Serialize, PartialEq)]
#[serde(untagged)]
#[non_exhaustive]
pub enum ZragChatContent {
    /// Plain-text message content.
    Text(String),
    /// Ordered multimodal content parts.
    Parts(Vec<ZragChatContentPart>),
}

impl ZragChatContent {
    /// Create plain-text content.
    pub fn text(text: impl Into<String>) -> Self {
        Self::Text(text.into())
    }

    /// Create multimodal content.
    pub fn parts(parts: Vec<ZragChatContentPart>) -> Self {
        Self::Parts(parts)
    }

    fn validate(&self) -> ZaiResult<()> {
        match self {
            Self::Text(text) => require_non_blank(text, "messages[].content"),
            Self::Parts(parts) => {
                if parts.is_empty() {
                    return Err(invalid(
                        "messages[].content parts must contain at least one item",
                    ));
                }
                for part in parts {
                    part.validate()?;
                }
                Ok(())
            },
        }
    }
}

impl From<String> for ZragChatContent {
    fn from(value: String) -> Self {
        Self::Text(value)
    }
}

impl From<&str> for ZragChatContent {
    fn from(value: &str) -> Self {
        Self::Text(value.to_owned())
    }
}

impl From<Vec<ZragChatContentPart>> for ZragChatContent {
    fn from(value: Vec<ZragChatContentPart>) -> Self {
        Self::Parts(value)
    }
}

impl std::fmt::Debug for ZragChatContent {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Text(_) => formatter.debug_tuple("Text").field(&"[REDACTED]").finish(),
            Self::Parts(parts) => formatter
                .debug_struct("Parts")
                .field("part_count", &parts.len())
                .finish(),
        }
    }
}

/// One text or image part in multimodal ZRAG chat content.
#[derive(Clone, Serialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
#[non_exhaustive]
pub enum ZragChatContentPart {
    /// A text content part.
    Text {
        /// Text sent to the agent.
        text: String,
    },
    /// An image URL content part.
    ImageUrl {
        /// Nested image URL object required by the wire schema.
        image_url: ZragChatImageUrl,
    },
}

impl ZragChatContentPart {
    /// Create a text part.
    pub fn text(text: impl Into<String>) -> Self {
        Self::Text { text: text.into() }
    }

    /// Create an image URL part.
    pub fn image_url(url: impl Into<String>) -> Self {
        Self::ImageUrl {
            image_url: ZragChatImageUrl::new(url),
        }
    }

    fn validate(&self) -> ZaiResult<()> {
        match self {
            Self::Text { text } => require_non_blank(text, "messages[].content[].text"),
            Self::ImageUrl { image_url } => image_url.validate(),
        }
    }
}

impl std::fmt::Debug for ZragChatContentPart {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Text { .. } => formatter
                .debug_struct("Text")
                .field("text", &"[REDACTED]")
                .finish(),
            Self::ImageUrl { .. } => formatter
                .debug_struct("ImageUrl")
                .field("url", &"[REDACTED]")
                .finish(),
        }
    }
}

/// Nested image URL object used by [`ZragChatContentPart::ImageUrl`].
#[derive(Clone, Serialize, PartialEq)]
pub struct ZragChatImageUrl {
    url: String,
}

impl ZragChatImageUrl {
    /// Create a nested image URL value.
    pub fn new(url: impl Into<String>) -> Self {
        Self { url: url.into() }
    }

    /// Borrow the image URL.
    pub fn url(&self) -> &str {
        &self.url
    }

    fn validate(&self) -> ZaiResult<()> {
        require_non_blank(&self.url, "messages[].content[].image_url.url")
    }
}

impl std::fmt::Debug for ZragChatImageUrl {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ZragChatImageUrl")
            .field("url", &"[REDACTED]")
            .finish()
    }
}

/// One user message accepted by ZRAG agent chat.
#[derive(Clone, Serialize, PartialEq)]
pub struct ZragChatMessage {
    role: ZragChatMessageRole,
    content: ZragChatContent,
}

impl ZragChatMessage {
    /// Create a user message from text or multimodal content.
    pub fn user(content: impl Into<ZragChatContent>) -> Self {
        Self {
            role: ZragChatMessageRole::User,
            content: content.into(),
        }
    }

    /// Return the fixed user role.
    pub const fn role(&self) -> ZragChatMessageRole {
        self.role
    }

    /// Borrow the message content.
    pub const fn content(&self) -> &ZragChatContent {
        &self.content
    }

    fn validate(&self) -> ZaiResult<()> {
        self.content.validate()
    }
}

impl std::fmt::Debug for ZragChatMessage {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ZragChatMessage")
            .field("role", &self.role)
            .field("content", &self.content)
            .finish()
    }
}

/// Message role supported by the ZRAG agent chat schema.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum ZragChatMessageRole {
    /// End-user input.
    User,
}

/// Retrieval preset used by a ZRAG agent chat request.
#[derive(Clone, Serialize, PartialEq)]
pub struct ZragChatRetrieval {
    know_ids: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_k: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_n: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    enable_rerank: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    similarity_threshold: Option<f64>,
}

impl ZragChatRetrieval {
    /// Select the knowledge bases available to the agent.
    pub fn new(knowledge_ids: Vec<String>) -> Self {
        Self {
            know_ids: knowledge_ids,
            top_k: None,
            top_n: None,
            enable_rerank: None,
            similarity_threshold: None,
        }
    }

    /// Set the retrieval count.
    pub fn with_top_k(mut self, top_k: u32) -> Self {
        self.top_k = Some(top_k);
        self
    }

    /// Set the recall count.
    pub fn with_top_n(mut self, top_n: u32) -> Self {
        self.top_n = Some(top_n);
        self
    }

    /// Enable or disable provider reranking.
    pub fn with_reranking(mut self, enabled: bool) -> Self {
        self.enable_rerank = Some(enabled);
        self
    }

    /// Set the provider similarity threshold.
    pub fn with_similarity_threshold(mut self, threshold: f64) -> Self {
        self.similarity_threshold = Some(threshold);
        self
    }

    /// Borrow the configured knowledge identifiers.
    pub fn knowledge_ids(&self) -> &[String] {
        &self.know_ids
    }

    fn validate(&self) -> ZaiResult<()> {
        if self.know_ids.is_empty() {
            return Err(invalid(
                "retrieval.know_ids must contain at least one knowledge base",
            ));
        }
        require_non_empty_strings(&self.know_ids, "retrieval.know_ids")?;
        if self.top_k == Some(0) {
            return Err(invalid("retrieval.top_k must be at least 1"));
        }
        if self.top_n == Some(0) {
            return Err(invalid("retrieval.top_n must be at least 1"));
        }
        if self
            .similarity_threshold
            .is_some_and(|threshold| !threshold.is_finite())
        {
            return Err(invalid("retrieval.similarity_threshold must be finite"));
        }
        Ok(())
    }
}

impl std::fmt::Debug for ZragChatRetrieval {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ZragChatRetrieval")
            .field("knowledge_id_count", &self.know_ids.len())
            .field("top_k", &self.top_k)
            .field("top_n", &self.top_n)
            .field("enable_rerank", &self.enable_rerank)
            .field("similarity_threshold", &self.similarity_threshold)
            .finish()
    }
}

/// Stream-only request for `POST /api/zrag/agent/chat`.
#[derive(Clone, Serialize, PartialEq)]
pub struct ZragChatRequest {
    messages: Vec<ZragChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_steps: Option<u32>,
    retrieval: ZragChatRetrieval,
    #[serde(skip_serializing_if = "Option::is_none")]
    enable_thinking: Option<bool>,
    #[serde(skip)]
    session_id: Option<String>,
}

impl ZragChatRequest {
    /// Create a chat request from the current user messages and retrieval preset.
    pub fn new(messages: Vec<ZragChatMessage>, retrieval: ZragChatRetrieval) -> Self {
        Self {
            messages,
            model: None,
            temperature: None,
            max_steps: None,
            retrieval,
            enable_thinking: None,
            session_id: None,
        }
    }

    /// Select the provider model instead of its default.
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Set the sampling temperature.
    pub fn with_temperature(mut self, temperature: f64) -> Self {
        self.temperature = Some(temperature);
        self
    }

    /// Set the maximum number of agent reasoning steps.
    pub fn with_max_steps(mut self, max_steps: u32) -> Self {
        self.max_steps = Some(max_steps);
        self
    }

    /// Enable or disable streamed reasoning events.
    pub fn with_thinking(mut self, enabled: bool) -> Self {
        self.enable_thinking = Some(enabled);
        self
    }

    /// Continue a provider session through the operation-local
    /// `X-Session-Id` header. The value never enters the JSON body and default
    /// [`Debug`](std::fmt::Debug) output records only whether it is configured.
    /// Treat the supplied value as a secret; [`Self::session_id`] returns it
    /// unredacted for explicit application use.
    pub fn with_session_id(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    /// Borrow the current messages.
    pub fn messages(&self) -> &[ZragChatMessage] {
        &self.messages
    }

    /// Borrow the retrieval preset.
    pub const fn retrieval(&self) -> &ZragChatRetrieval {
        &self.retrieval
    }

    /// Borrow the optional continuation session identifier without redaction.
    ///
    /// Treat the returned value as a secret. The transport removes exact
    /// echoes from response diagnostics, but this explicit accessor returns
    /// the original value.
    pub fn session_id(&self) -> Option<&str> {
        self.session_id.as_deref()
    }

    /// Validate the request and its operation-local header without network I/O.
    pub fn validate(&self) -> ZaiResult<()> {
        if self.messages.is_empty() {
            return Err(invalid("messages must contain at least one user message"));
        }
        for message in &self.messages {
            message.validate()?;
        }
        self.retrieval.validate()?;
        if self
            .model
            .as_deref()
            .is_some_and(|model| model.trim().is_empty())
        {
            return Err(invalid("model must not be blank when provided"));
        }
        if self
            .temperature
            .is_some_and(|temperature| !temperature.is_finite())
        {
            return Err(invalid("temperature must be finite"));
        }
        if self.max_steps == Some(0) {
            return Err(invalid("max_steps must be at least 1"));
        }
        if let Some(session_id) = &self.session_id {
            crate::client::transport::request::SensitiveHeader::new(SESSION_HEADER, session_id)?;
        }
        Ok(())
    }

    /// Validate, dispatch, and return the typed ZRAG SSE stream.
    ///
    /// The handshake accepts only an unranged `200 OK` with
    /// `text/event-stream`, and this streaming POST is never retried or
    /// redirected. Successful completion yields its
    /// [`AgentStreamEvent::Done`] item before the stream terminates.
    pub async fn stream_via(&self, client: &ZaiClient) -> ZaiResult<ZragEventStream> {
        self.validate()?;
        let mut operation = client.operation(crate::client::routes::ZRAG_CHAT);
        if let Some(session_id) = &self.session_id {
            operation = operation.with_sensitive_header(SESSION_HEADER, session_id)?;
        }
        let raw = operation.send_sse_json(self).await?;
        Ok(ZragEventStream {
            inner: decode_zrag_stream(raw, self.session_id.clone()),
        })
    }

    /// Alias for [`Self::stream_via`] matching the crate-wide request-centric
    /// `send_via` convention. This endpoint remains stream-only.
    pub async fn send_via(&self, client: &ZaiClient) -> ZaiResult<ZragEventStream> {
        self.stream_via(client).await
    }
}

impl std::fmt::Debug for ZragChatRequest {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ZragChatRequest")
            .field("message_count", &self.messages.len())
            .field("model_configured", &self.model.is_some())
            .field("temperature", &self.temperature)
            .field("max_steps", &self.max_steps)
            .field("retrieval", &self.retrieval)
            .field("enable_thinking", &self.enable_thinking)
            .field("session_id_configured", &self.session_id.is_some())
            .finish()
    }
}

/// Text payload shared by `session_created`, `reasoning`, `thought`, and
/// `answer` stream events.
#[derive(Clone, PartialEq)]
pub struct AgentTextEvent {
    session_id: Option<String>,
    data: Option<String>,
}

impl AgentTextEvent {
    /// Borrow the optional provider session identifier without redaction.
    pub fn session_id(&self) -> Option<&str> {
        self.session_id.as_deref()
    }

    /// Borrow the optional text payload without redaction.
    pub fn data(&self) -> Option<&str> {
        self.data.as_deref()
    }
}

impl std::fmt::Debug for AgentTextEvent {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AgentTextEvent")
            .field("session_id_configured", &self.session_id.is_some())
            .field("data_configured", &self.data.is_some())
            .finish()
    }
}

/// Typed `tool_call` event data.
#[derive(Clone, PartialEq, Deserialize)]
pub struct AgentToolCallData {
    #[serde(default, rename = "callId")]
    call_id: Option<String>,
    #[serde(default, rename = "toolName")]
    tool_name: Option<String>,
    #[serde(default)]
    arguments: Option<Map<String, Value>>,
}

impl AgentToolCallData {
    /// Borrow the optional provider call identifier without redaction.
    pub fn call_id(&self) -> Option<&str> {
        self.call_id.as_deref()
    }

    /// Borrow the optional provider tool name without redaction.
    pub fn tool_name(&self) -> Option<&str> {
        self.tool_name.as_deref()
    }

    /// Borrow the open argument object without redaction.
    ///
    /// Arguments can contain user or tool data and require an
    /// application-specific content policy before logging.
    pub const fn arguments(&self) -> Option<&Map<String, Value>> {
        self.arguments.as_ref()
    }
}

impl std::fmt::Debug for AgentToolCallData {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AgentToolCallData")
            .field("call_id_configured", &self.call_id.is_some())
            .field("tool_name_configured", &self.tool_name.is_some())
            .field("argument_count", &self.arguments.as_ref().map(Map::len))
            .finish()
    }
}

/// Provider status for a `tool_result` event.
#[derive(Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum AgentToolResultStatus {
    /// Tool execution succeeded.
    Success,
    /// Tool execution failed.
    Error,
    /// A status added by a future provider version, retained verbatim.
    Unknown(String),
}

impl AgentToolResultStatus {
    /// Borrow the exact, unredacted wire value.
    ///
    /// Future provider values retained by [`Self::Unknown`] require an
    /// application-specific content policy before logging.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Success => "success",
            Self::Error => "error",
            Self::Unknown(value) => value,
        }
    }
}

impl<'de> Deserialize<'de> for AgentToolResultStatus {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Ok(match value.as_str() {
            "success" => Self::Success,
            "error" => Self::Error,
            _ => Self::Unknown(value),
        })
    }
}

impl std::fmt::Debug for AgentToolResultStatus {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Success => formatter.write_str("Success"),
            Self::Error => formatter.write_str("Error"),
            Self::Unknown(_) => formatter
                .debug_tuple("Unknown")
                .field(&"[REDACTED]")
                .finish(),
        }
    }
}

/// Typed `tool_result` event data.
#[derive(Clone, PartialEq, Deserialize)]
pub struct AgentToolResultData {
    #[serde(default, rename = "callId")]
    call_id: Option<String>,
    #[serde(default, rename = "toolName")]
    tool_name: Option<String>,
    #[serde(default)]
    result: Option<Value>,
    #[serde(default)]
    status: Option<AgentToolResultStatus>,
    #[serde(default, rename = "durationMs")]
    duration_ms: Option<i64>,
}

impl AgentToolResultData {
    /// Borrow the optional provider call identifier without redaction.
    pub fn call_id(&self) -> Option<&str> {
        self.call_id.as_deref()
    }

    /// Borrow the optional provider tool name without redaction.
    pub fn tool_name(&self) -> Option<&str> {
        self.tool_name.as_deref()
    }

    /// Borrow the provider's open result value without redaction.
    ///
    /// Results can contain user or tool data and require an
    /// application-specific content policy before logging.
    pub const fn result(&self) -> Option<&Value> {
        self.result.as_ref()
    }

    /// Return the optional execution status. A future status retains its
    /// exact, unredacted wire value.
    pub const fn status(&self) -> Option<&AgentToolResultStatus> {
        self.status.as_ref()
    }

    /// Return the optional execution duration in milliseconds.
    pub const fn duration_ms(&self) -> Option<i64> {
        self.duration_ms
    }
}

impl std::fmt::Debug for AgentToolResultData {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AgentToolResultData")
            .field("call_id_configured", &self.call_id.is_some())
            .field("tool_name_configured", &self.tool_name.is_some())
            .field("result_configured", &self.result.is_some())
            .field("status", &self.status)
            .field("duration_ms", &self.duration_ms)
            .finish()
    }
}

/// Token details nested under prompt usage.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct AgentPromptTokenDetails {
    #[serde(default)]
    cached_tokens: Option<i64>,
}

impl AgentPromptTokenDetails {
    /// Return cached prompt tokens, when reported.
    pub const fn cached_tokens(&self) -> Option<i64> {
        self.cached_tokens
    }
}

/// Token details nested under completion usage.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct AgentCompletionTokenDetails {
    #[serde(default)]
    reasoning_tokens: Option<i64>,
}

impl AgentCompletionTokenDetails {
    /// Return reasoning tokens, when reported.
    pub const fn reasoning_tokens(&self) -> Option<i64> {
        self.reasoning_tokens
    }
}

/// Token and tool-call usage reported by the terminal event.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct AgentUsage {
    #[serde(default)]
    prompt_tokens: Option<i64>,
    #[serde(default)]
    completion_tokens: Option<i64>,
    #[serde(default)]
    total_tokens: Option<i64>,
    #[serde(default)]
    total_calls: Option<i64>,
    #[serde(default)]
    prompt_tokens_details: Option<AgentPromptTokenDetails>,
    #[serde(default)]
    completion_tokens_details: Option<AgentCompletionTokenDetails>,
}

impl AgentUsage {
    /// Return prompt tokens, when reported.
    pub const fn prompt_tokens(&self) -> Option<i64> {
        self.prompt_tokens
    }

    /// Return completion tokens, when reported.
    pub const fn completion_tokens(&self) -> Option<i64> {
        self.completion_tokens
    }

    /// Return total tokens, when reported.
    pub const fn total_tokens(&self) -> Option<i64> {
        self.total_tokens
    }

    /// Return total tool calls, when reported.
    pub const fn total_calls(&self) -> Option<i64> {
        self.total_calls
    }

    /// Borrow optional prompt-token details.
    pub const fn prompt_token_details(&self) -> Option<&AgentPromptTokenDetails> {
        self.prompt_tokens_details.as_ref()
    }

    /// Borrow optional completion-token details.
    pub const fn completion_token_details(&self) -> Option<&AgentCompletionTokenDetails> {
        self.completion_tokens_details.as_ref()
    }
}

/// Typed `tool_call` event.
#[derive(Clone, PartialEq)]
pub struct AgentToolCallEvent {
    session_id: Option<String>,
    data: Option<AgentToolCallData>,
}

impl AgentToolCallEvent {
    /// Borrow the optional provider session identifier without redaction.
    pub fn session_id(&self) -> Option<&str> {
        self.session_id.as_deref()
    }

    /// Borrow the optional typed call data.
    pub const fn data(&self) -> Option<&AgentToolCallData> {
        self.data.as_ref()
    }
}

impl std::fmt::Debug for AgentToolCallEvent {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AgentToolCallEvent")
            .field("session_id_configured", &self.session_id.is_some())
            .field("data", &self.data)
            .finish()
    }
}

/// Typed `tool_result` event.
#[derive(Clone, PartialEq)]
pub struct AgentToolResultEvent {
    session_id: Option<String>,
    data: Option<AgentToolResultData>,
}

impl AgentToolResultEvent {
    /// Borrow the optional provider session identifier without redaction.
    pub fn session_id(&self) -> Option<&str> {
        self.session_id.as_deref()
    }

    /// Borrow the optional typed result data.
    pub const fn data(&self) -> Option<&AgentToolResultData> {
        self.data.as_ref()
    }
}

impl std::fmt::Debug for AgentToolResultEvent {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AgentToolResultEvent")
            .field("session_id_configured", &self.session_id.is_some())
            .field("data", &self.data)
            .finish()
    }
}

/// Typed terminal `done` event.
#[derive(Clone, PartialEq)]
pub struct AgentDoneEvent {
    session_id: Option<String>,
    message_id: Option<String>,
    usage: Option<AgentUsage>,
}

impl AgentDoneEvent {
    /// Borrow the optional provider session identifier without redaction.
    pub fn session_id(&self) -> Option<&str> {
        self.session_id.as_deref()
    }

    /// Borrow the optional provider message identifier without redaction.
    pub fn message_id(&self) -> Option<&str> {
        self.message_id.as_deref()
    }

    /// Borrow terminal token usage.
    pub const fn usage(&self) -> Option<&AgentUsage> {
        self.usage.as_ref()
    }
}

impl std::fmt::Debug for AgentDoneEvent {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AgentDoneEvent")
            .field("session_id_configured", &self.session_id.is_some())
            .field("message_id_configured", &self.message_id.is_some())
            .field("usage", &self.usage)
            .finish()
    }
}

/// Forward-compatible representation of an unrecognized event type.
#[derive(Clone, PartialEq)]
pub struct AgentUnknownEvent {
    event_type: String,
    raw: Value,
}

impl AgentUnknownEvent {
    /// Borrow the unrecognized provider event type.
    ///
    /// This explicit accessor is not redacted and may contain provider data.
    pub fn event_type(&self) -> &str {
        &self.event_type
    }

    /// Borrow the complete raw JSON event for forward-compatible handling.
    ///
    /// The value is intentionally unredacted and can contain user content,
    /// tool arguments, session identifiers, or other sensitive application
    /// data. Do not log it without an application-specific content policy.
    pub const fn raw(&self) -> &Value {
        &self.raw
    }
}

impl std::fmt::Debug for AgentUnknownEvent {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AgentUnknownEvent")
            .field("event_type", &"[REDACTED]")
            .field("raw", &"[REDACTED]")
            .finish()
    }
}

/// One decoded ZRAG agent stream event.
///
/// A provider `type=error` payload is yielded as `Err`, not as an enum variant.
/// Unknown future event types are retained by [`Self::Unknown`].
#[derive(Clone, PartialEq)]
#[non_exhaustive]
pub enum AgentStreamEvent {
    /// A provider session was created.
    SessionCreated(AgentTextEvent),
    /// One streamed reasoning fragment.
    Reasoning(AgentTextEvent),
    /// One streamed thought fragment.
    Thought(AgentTextEvent),
    /// One tool invocation.
    ToolCall(AgentToolCallEvent),
    /// One tool result.
    ToolResult(AgentToolResultEvent),
    /// One final-answer fragment.
    Answer(AgentTextEvent),
    /// The typed terminal event. It is yielded once before normal termination.
    Done(AgentDoneEvent),
    /// An event type added by a future provider version.
    Unknown(AgentUnknownEvent),
}

impl AgentStreamEvent {
    /// Return whether this is the terminal `type=done` event.
    pub const fn is_done(&self) -> bool {
        matches!(self, Self::Done(_))
    }

    /// Borrow a provider session identifier carried by a known event without
    /// redaction.
    pub fn session_id(&self) -> Option<&str> {
        match self {
            Self::SessionCreated(event)
            | Self::Reasoning(event)
            | Self::Thought(event)
            | Self::Answer(event) => event.session_id(),
            Self::ToolCall(event) => event.session_id(),
            Self::ToolResult(event) => event.session_id(),
            Self::Done(event) => event.session_id(),
            Self::Unknown(_) => None,
        }
    }
}

impl std::fmt::Debug for AgentStreamEvent {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SessionCreated(event) => formatter
                .debug_tuple("SessionCreated")
                .field(event)
                .finish(),
            Self::Reasoning(event) => formatter.debug_tuple("Reasoning").field(event).finish(),
            Self::Thought(event) => formatter.debug_tuple("Thought").field(event).finish(),
            Self::ToolCall(event) => formatter.debug_tuple("ToolCall").field(event).finish(),
            Self::ToolResult(event) => formatter.debug_tuple("ToolResult").field(event).finish(),
            Self::Answer(event) => formatter.debug_tuple("Answer").field(event).finish(),
            Self::Done(event) => formatter.debug_tuple("Done").field(event).finish(),
            Self::Unknown(event) => formatter.debug_tuple("Unknown").field(event).finish(),
        }
    }
}

/// Authenticated typed stream returned by [`ZragChatRequest::stream_via`].
pub struct ZragEventStream {
    inner: Pin<Box<dyn Stream<Item = ZaiResult<AgentStreamEvent>> + Send + 'static>>,
}

impl ZragEventStream {
    /// Await the next typed event, terminal error, or end of stream.
    pub async fn next(&mut self) -> Option<ZaiResult<AgentStreamEvent>> {
        self.inner.next().await
    }
}

impl Stream for ZragEventStream {
    type Item = ZaiResult<AgentStreamEvent>;

    fn poll_next(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.inner.as_mut().poll_next(context)
    }
}

impl std::fmt::Debug for ZragEventStream {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ZragEventStream")
            .finish_non_exhaustive()
    }
}

/// Descriptive alias for [`AgentStreamEvent`].
pub type ZragChatEvent = AgentStreamEvent;

/// Descriptive alias for [`ZragEventStream`].
pub type ZragChatStream = ZragEventStream;

#[derive(Deserialize)]
struct WireEvent {
    #[serde(rename = "type")]
    event_type: String,
    #[serde(default, rename = "sessionId")]
    session_id: Option<String>,
    #[serde(default, rename = "messageId")]
    message_id: Option<String>,
    #[serde(default)]
    data: Option<Value>,
    #[serde(default)]
    usage: Option<AgentUsage>,
}

#[derive(Deserialize)]
struct WireErrorData {
    #[serde(default)]
    message: Option<String>,
}

struct ZragDecodeState {
    raw: Option<crate::client::transport::SseByteStream>,
    parser: crate::model::sse_parser::SseEventParser,
    current_chunk: Option<bytes::Bytes>,
    chunk_offset: usize,
    input_finished: bool,
    terminated: bool,
    session_secret: Option<String>,
}

fn decode_zrag_stream(
    raw: crate::client::transport::SseByteStream,
    session_secret: Option<String>,
) -> Pin<Box<dyn Stream<Item = ZaiResult<AgentStreamEvent>> + Send + 'static>> {
    let state = ZragDecodeState::new(raw, session_secret);
    Box::pin(futures_util::stream::unfold(
        state,
        ZragDecodeState::next_item,
    ))
}

impl ZragDecodeState {
    fn new(raw: crate::client::transport::SseByteStream, session_secret: Option<String>) -> Self {
        Self {
            raw: Some(raw),
            parser: crate::model::sse_parser::SseEventParser::new(),
            current_chunk: None,
            chunk_offset: 0,
            input_finished: false,
            terminated: false,
            session_secret,
        }
    }

    async fn next_item(mut self) -> Option<(ZaiResult<AgentStreamEvent>, Self)> {
        loop {
            if self.terminated {
                return None;
            }

            let next_payload = if self.input_finished {
                self.parser.finish_next_bounded()
            } else {
                self.parser.next_bounded()
            };
            let payload = match next_payload {
                Ok(payload) => payload,
                Err(error) => {
                    self.terminate();
                    return Some((Err(error), self));
                },
            };

            if let Some(payload) = payload {
                let item = decode_event(&payload, self.session_secret.as_deref());
                let terminal = item.as_ref().map_or(true, AgentStreamEvent::is_done);
                if terminal {
                    self.terminate();
                }
                return Some((item, self));
            }

            if self.input_finished {
                self.terminate();
                return Some((Err(ended_without_done()), self));
            }

            if self.parser.buffered_len()
                > crate::client::transport::limits::SSE_PARSER_RETAINED_MAX
            {
                self.terminate();
                return Some((Err(sse_event_too_large()), self));
            }

            if let Some(chunk) = self.current_chunk.take() {
                let end = self
                    .chunk_offset
                    .saturating_add(crate::client::transport::limits::SSE_PARSE_SLICE_BYTES)
                    .min(chunk.len());
                self.parser.feed(&chunk[self.chunk_offset..end]);
                if end < chunk.len() {
                    self.current_chunk = Some(chunk);
                    self.chunk_offset = end;
                } else {
                    self.chunk_offset = 0;
                }
                continue;
            }

            let Some(raw) = self.raw.as_mut() else {
                self.terminate();
                return Some((Err(ended_without_done()), self));
            };
            match raw.next().await {
                Some(Ok(chunk)) if chunk.is_empty() => {},
                Some(Ok(chunk)) => self.current_chunk = Some(chunk),
                Some(Err(error)) => {
                    self.terminate();
                    return Some((Err(error), self));
                },
                None => self.input_finished = true,
            }
        }
    }

    fn terminate(&mut self) {
        self.terminated = true;
        self.raw = None;
        self.current_chunk = None;
        self.chunk_offset = 0;
        self.parser = crate::model::sse_parser::SseEventParser::new();
        self.session_secret = None;
    }
}

fn decode_event(payload: &[u8], session_secret: Option<&str>) -> ZaiResult<AgentStreamEvent> {
    if payload == b"[DONE]" {
        return Err(protocol_error(
            "ZRAG requires a JSON type=done event; literal [DONE] is invalid",
        ));
    }
    let raw = serde_json::from_slice::<UniqueJsonValue>(payload)
        .map(UniqueJsonValue::into_inner)
        .map_err(|_| protocol_error("ZRAG SSE data must be unique-key JSON"))?;
    if !raw.is_object() {
        return Err(protocol_error("ZRAG SSE event must be a JSON object"));
    }
    let Some(event_type) = raw.get("type").and_then(Value::as_str) else {
        return Err(protocol_error(
            "ZRAG SSE data must be a JSON object with a string type field",
        ));
    };
    let known_event = matches!(
        event_type,
        "session_created"
            | "reasoning"
            | "thought"
            | "tool_call"
            | "tool_result"
            | "answer"
            | "done"
            | "error"
    );
    if !known_event {
        let event_type = event_type.to_owned();
        return Ok(AgentStreamEvent::Unknown(AgentUnknownEvent {
            event_type,
            raw,
        }));
    }
    let wire: WireEvent = serde_json::from_value(raw)
        .map_err(|_| protocol_error("ZRAG SSE event has invalid documented field types"))?;

    let WireEvent {
        event_type,
        session_id,
        data,
        message_id,
        usage,
    } = wire;
    match event_type.as_str() {
        "session_created" => Ok(AgentStreamEvent::SessionCreated(decode_text_event(
            session_id, data,
        )?)),
        "reasoning" => Ok(AgentStreamEvent::Reasoning(decode_text_event(
            session_id, data,
        )?)),
        "thought" => Ok(AgentStreamEvent::Thought(decode_text_event(
            session_id, data,
        )?)),
        "answer" => Ok(AgentStreamEvent::Answer(decode_text_event(
            session_id, data,
        )?)),
        "tool_call" => Ok(AgentStreamEvent::ToolCall(AgentToolCallEvent {
            session_id,
            data: decode_optional(data, "ZRAG tool_call data must be an object")?,
        })),
        "tool_result" => Ok(AgentStreamEvent::ToolResult(AgentToolResultEvent {
            session_id,
            data: decode_optional(data, "ZRAG tool_result data must be an object")?,
        })),
        "done" => {
            if data
                .as_ref()
                .is_some_and(|data| !data.is_null() && !data.is_string() && !data.is_object())
            {
                return Err(protocol_error(
                    "ZRAG done event data must match the documented string-or-object union",
                ));
            }
            Ok(AgentStreamEvent::Done(AgentDoneEvent {
                session_id,
                message_id,
                usage,
            }))
        },
        "error" => {
            let data: Option<WireErrorData> =
                decode_optional(data, "ZRAG error event data must be an object")?;
            let message = data
                .and_then(|data| data.message)
                .filter(|message| !message.trim().is_empty())
                .unwrap_or_else(|| "ZRAG stream reported an error".to_string());
            Err(ZaiError::ApiError {
                code: codes::SDK_IO,
                message: redact_session(&message, session_secret),
            })
        },
        _ => unreachable!("known ZRAG event type was exhaustively matched"),
    }
}

fn decode_text_event(session_id: Option<String>, data: Option<Value>) -> ZaiResult<AgentTextEvent> {
    Ok(AgentTextEvent {
        session_id,
        data: decode_optional(data, "ZRAG text event data must be a string")?,
    })
}

fn decode_optional<T: serde::de::DeserializeOwned>(
    value: Option<Value>,
    message: &'static str,
) -> ZaiResult<Option<T>> {
    value
        .filter(|value| !value.is_null())
        .map(|value| serde_json::from_value(value).map_err(|_| protocol_error(message)))
        .transpose()
}

fn redact_session(message: &str, session_secret: Option<&str>) -> String {
    let message = crate::client::error::mask_sensitive_info(message);
    match session_secret.filter(|secret| !secret.is_empty()) {
        Some(secret) => message.replace(secret, "[REDACTED]"),
        None => message,
    }
}

fn ended_without_done() -> ZaiError {
    ZaiError::ApiError {
        code: codes::SDK_IO,
        message: "ZRAG SSE stream ended before the required type=done event".to_string(),
    }
}

fn sse_event_too_large() -> ZaiError {
    ZaiError::ApiError {
        code: codes::SDK_VALIDATION,
        message: format!(
            "SSE event exceeded limit ({} bytes)",
            crate::client::transport::limits::SSE_EVENT_BYTES_MAX
        ),
    }
}

fn protocol_error(message: &'static str) -> ZaiError {
    ZaiError::ApiError {
        code: codes::SDK_VALIDATION,
        message: message.to_string(),
    }
}

fn invalid(message: impl Into<String>) -> ZaiError {
    ZaiError::ApiError {
        code: codes::SDK_VALIDATION,
        message: message.into(),
    }
}

fn require_non_blank(value: &str, field: &str) -> ZaiResult<()> {
    if value.trim().is_empty() {
        return Err(invalid(format!("{field} must not be blank")));
    }
    Ok(())
}

fn require_non_empty_strings(values: &[String], field: &str) -> ZaiResult<()> {
    if values.iter().any(|value| value.trim().is_empty()) {
        return Err(invalid(format!("{field} cannot contain blank values")));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::stream;
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };

    struct DropTrackedStream {
        chunk: Option<bytes::Bytes>,
        dropped: Arc<AtomicBool>,
    }

    impl Stream for DropTrackedStream {
        type Item = ZaiResult<bytes::Bytes>;

        fn poll_next(
            mut self: Pin<&mut Self>,
            _context: &mut Context<'_>,
        ) -> Poll<Option<Self::Item>> {
            Poll::Ready(self.chunk.take().map(Ok))
        }
    }

    impl Drop for DropTrackedStream {
        fn drop(&mut self) {
            self.dropped.store(true, Ordering::SeqCst);
        }
    }

    fn raw(body: &'static [u8]) -> crate::client::transport::SseByteStream {
        Box::pin(stream::iter([Ok(bytes::Bytes::from_static(body))]))
    }

    fn tracked_raw(
        body: Vec<u8>,
        dropped: Arc<AtomicBool>,
    ) -> crate::client::transport::SseByteStream {
        Box::pin(DropTrackedStream {
            chunk: Some(bytes::Bytes::from(body)),
            dropped,
        })
    }

    #[test]
    fn request_serialization_and_debug_keep_session_out_of_the_body_and_logs() {
        let request = ZragChatRequest::new(
            vec![ZragChatMessage::user(vec![
                ZragChatContentPart::text("private question"),
                ZragChatContentPart::image_url("https://private.example/image.png"),
            ])],
            ZragChatRetrieval::new(vec!["private-knowledge".to_string()]),
        )
        .with_session_id("private-session")
        .with_model("private-model");

        let json = serde_json::to_value(&request).unwrap();
        assert!(json.get("session_id").is_none());
        assert_eq!(json["messages"][0]["role"], "user");
        assert_eq!(json["messages"][0]["content"][1]["type"], "image_url");

        let debug = format!("{request:?}");
        for secret in [
            "private question",
            "https://private.example/image.png",
            "private-knowledge",
            "private-session",
            "private-model",
        ] {
            assert!(!debug.contains(secret));
        }
    }

    #[test]
    fn validation_covers_required_values_and_header_syntax() {
        let retrieval = || ZragChatRetrieval::new(vec!["knowledge".to_string()]);
        for request in [
            ZragChatRequest::new(Vec::new(), retrieval()),
            ZragChatRequest::new(vec![ZragChatMessage::user(" ")], retrieval()),
            ZragChatRequest::new(
                vec![ZragChatMessage::user(Vec::<ZragChatContentPart>::new())],
                retrieval(),
            ),
            ZragChatRequest::new(
                vec![ZragChatMessage::user("question")],
                ZragChatRetrieval::new(Vec::new()),
            ),
            ZragChatRequest::new(vec![ZragChatMessage::user("question")], retrieval())
                .with_max_steps(0),
            ZragChatRequest::new(vec![ZragChatMessage::user("question")], retrieval())
                .with_temperature(f64::NAN),
            ZragChatRequest::new(vec![ZragChatMessage::user("question")], retrieval())
                .with_session_id("secret session"),
        ] {
            assert!(request.validate().is_err());
        }
    }

    #[tokio::test]
    async fn decoder_yields_done_then_stops_and_preserves_unknown_events() {
        let body = b"data: {\"type\":\"future_event\",\"future\":{\"x\":1}}\n\ndata: {\"type\":\"answer\",\"data\":\"hello\"}\n\ndata: {\"type\":\"done\",\"messageId\":\"message\",\"usage\":{\"total_tokens\":3}}\n\ndata: {\"type\":\"answer\",\"data\":\"ignored\"}\n\n";
        let mut decoded = decode_zrag_stream(raw(body), None);

        let AgentStreamEvent::Unknown(unknown) = decoded.next().await.unwrap().unwrap() else {
            panic!("expected unknown event");
        };
        assert_eq!(unknown.event_type(), "future_event");
        assert_eq!(unknown.raw()["future"]["x"], 1);
        let AgentStreamEvent::Answer(answer) = decoded.next().await.unwrap().unwrap() else {
            panic!("expected answer event");
        };
        assert_eq!(answer.data(), Some("hello"));
        let done = decoded.next().await.unwrap().unwrap();
        assert!(done.is_done());
        assert!(decoded.next().await.is_none());
    }

    #[tokio::test]
    async fn decoder_errors_once_for_error_eof_and_literal_done() {
        for body in [
            b"data: {\"type\":\"error\",\"data\":{\"message\":\"failed\"}}\n\n".as_slice(),
            b"data: {\"type\":\"answer\",\"data\":\"partial\"}\n\n".as_slice(),
            b"data: [DONE]\n\n".as_slice(),
        ] {
            let mut decoded = decode_zrag_stream(
                Box::pin(stream::iter([Ok(bytes::Bytes::copy_from_slice(body))])),
                None,
            );
            let first = decoded.next().await.unwrap();
            if first.is_ok() {
                assert!(decoded.next().await.unwrap().is_err());
            } else {
                assert!(first.is_err());
            }
            assert!(decoded.next().await.is_none());
        }
    }

    #[tokio::test]
    async fn done_precedes_and_discards_an_over_limit_tail_in_the_same_chunk() {
        let dropped = Arc::new(AtomicBool::new(false));
        let mut body = b"data: {\"type\":\"done\"}\n\n".to_vec();
        for _ in 0..=crate::client::transport::limits::SSE_EVENT_DATA_LINES_MAX {
            body.extend_from_slice(b"data: x\n");
        }
        body.push(b'\n');

        let mut decoded = decode_zrag_stream(tracked_raw(body, Arc::clone(&dropped)), None);
        assert!(decoded.next().await.unwrap().unwrap().is_done());
        assert!(
            dropped.load(Ordering::SeqCst),
            "yielding done must release the raw authenticated stream immediately"
        );
        assert!(decoded.next().await.is_none());
    }

    #[tokio::test]
    async fn decoder_feeds_at_most_one_parse_slice_before_each_event() {
        let event = b"data: {\"type\":\"answer\",\"data\":\"x\"}\n\n";
        let mut body = Vec::new();
        while body.len() <= crate::client::transport::limits::SSE_PARSE_SLICE_BYTES * 2 {
            body.extend_from_slice(event);
        }
        body.extend_from_slice(b"data: {\"type\":\"done\"}\n\n");

        let raw: crate::client::transport::SseByteStream =
            Box::pin(stream::iter([Ok(bytes::Bytes::from(body))]));
        let state = ZragDecodeState::new(raw, None);
        let (first, state) = state.next_item().await.unwrap();
        assert!(matches!(first.unwrap(), AgentStreamEvent::Answer(_)));
        assert_eq!(
            state.chunk_offset,
            crate::client::transport::limits::SSE_PARSE_SLICE_BYTES
        );
        assert!(state.current_chunk.is_some());
        assert!(
            state.parser.buffered_len() <= crate::client::transport::limits::SSE_PARSE_SLICE_BYTES
        );
    }

    #[tokio::test]
    async fn terminal_error_drops_raw_without_requiring_another_poll() {
        let dropped = Arc::new(AtomicBool::new(false));
        let body = b"data: {\"type\":\"error\",\"data\":{\"message\":\"failed\"}}\n\n".to_vec();
        let mut decoded = decode_zrag_stream(tracked_raw(body, Arc::clone(&dropped)), None);

        assert!(decoded.next().await.unwrap().is_err());
        assert!(
            dropped.load(Ordering::SeqCst),
            "yielding a terminal error must release the raw stream immediately"
        );
    }

    #[tokio::test]
    async fn duplicate_json_keys_fail_once_for_known_and_unknown_events() {
        for body in [
            b"data: {\"type\":\"answer\",\"type\":\"done\"}\n\n".as_slice(),
            b"data: {\"type\":\"answer\",\"type\":\"answer\",\"data\":\"private-duplicate-value\"}\n\n"
                .as_slice(),
            b"data: {\"type\":\"tool_call\",\"data\":{\"arguments\":{\"q\":1,\"q\":2}}}\n\n"
                .as_slice(),
            b"data: {\"type\":\"tool_call\",\"data\":{\"arguments\":{\"q\":1,\"q\":1}}}\n\n"
                .as_slice(),
            b"data: {\"type\":\"tool_result\",\"data\":{\"status\":\"success\",\"status\":\"error\"}}\n\n"
                .as_slice(),
            b"data: {\"type\":\"future_event\",\"future\":{\"x\":1,\"x\":2}}\n\n"
                .as_slice(),
            b"data: {\"type\":\"future_event\",\"future\":{\"x\":1,\"x\":1}}\n\n"
                .as_slice(),
        ] {
            let raw: crate::client::transport::SseByteStream =
                Box::pin(stream::iter([Ok(bytes::Bytes::copy_from_slice(body))]));
            let mut decoded = decode_zrag_stream(raw, None);
            let error = decoded.next().await.unwrap().unwrap_err();
            assert_eq!(error.code(), Some(codes::SDK_VALIDATION));
            assert_eq!(error.message(), "ZRAG SSE data must be unique-key JSON");
            assert!(!error.message().contains("private-duplicate-value"));
            assert!(decoded.next().await.is_none());
        }
    }

    #[test]
    fn known_event_preserves_a_large_nested_result_payload() {
        let rows = (0..4_096)
            .map(|index| {
                serde_json::json!({
                    "index": index,
                    "cells": [index, index + 1, index + 2],
                    "metadata": {"label": format!("row-{index}")}
                })
            })
            .collect::<Vec<_>>();
        let payload = serde_json::json!({
            "type": "tool_result",
            "sessionId": "session-large",
            "data": {"status": "future_status", "result": {"rows": rows}}
        });
        let encoded = serde_json::to_vec(&payload).unwrap();
        assert!(
            encoded.len() > crate::client::transport::limits::SSE_PARSE_SLICE_BYTES,
            "regression fixture must span multiple parser slices"
        );

        let AgentStreamEvent::ToolResult(event) = decode_event(&encoded, None).unwrap() else {
            panic!("large known event routed to the wrong variant");
        };
        assert_eq!(event.session_id(), Some("session-large"));
        let result = event.data().and_then(AgentToolResultData::result).unwrap();
        let decoded_rows = result["rows"].as_array().unwrap();
        assert_eq!(decoded_rows.len(), 4_096);
        assert_eq!(decoded_rows[4_095]["metadata"]["label"], "row-4095");
        assert_eq!(
            event.data().and_then(AgentToolResultData::status),
            Some(&AgentToolResultStatus::Unknown("future_status".to_owned()))
        );
    }
}
