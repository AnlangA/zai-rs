//! # Agent v1 API
//!
//! Strongly typed request/response values and [`ZaiClient`]
//! dispatch for the three frozen Agent v1 operations:
//!
//! - `POST /v1/agents`
//! - `POST /v1/agents/async-result`
//! - `POST /v1/agents/conversation`
//!
//! [`AgentInvokeRequest<NonStreaming>`] supports the JSON response path. The
//! [`Streaming`] type-state intentionally has no `send_via` method until a typed
//! Agent streaming decoder is available.

mod request;
mod response;

pub use request::*;
pub use response::*;

use std::marker::PhantomData;

use serde::{Deserialize, Serialize};

use crate::{
    ZaiResult,
    client::{ZaiClient, validation::require_non_blank},
};

/// Open variables object accepted by the Agent invocation schema.
///
/// This is the only open JSON map in the Agent v1 request contracts.
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AgentCustomVariables(serde_json::Map<String, serde_json::Value>);

impl std::fmt::Debug for AgentCustomVariables {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AgentCustomVariables")
            .field("keys", &self.0.keys().collect::<Vec<_>>())
            .field("values", &"[REDACTED]")
            .finish()
    }
}

impl AgentCustomVariables {
    /// Construct an empty variables object.
    pub fn new() -> Self {
        Self(serde_json::Map::new())
    }

    /// Insert an open-schema variable value.
    pub fn insert(
        &mut self,
        key: impl Into<String>,
        value: serde_json::Value,
    ) -> Option<serde_json::Value> {
        self.0.insert(key.into(), value)
    }

    /// Return whether no variables are present.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Borrow the underlying open map.
    pub const fn as_map(&self) -> &serde_json::Map<String, serde_json::Value> {
        &self.0
    }
}

impl From<serde_json::Map<String, serde_json::Value>> for AgentCustomVariables {
    fn from(value: serde_json::Map<String, serde_json::Value>) -> Self {
        Self(value)
    }
}

/// Type-state marker for a non-streaming Agent invocation.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct NonStreaming;

/// Type-state marker for a streaming Agent invocation.
///
/// Streaming requests deliberately have no JSON `send_via` path:
///
/// ```compile_fail
/// use zai_rs::{
///     ZaiClient,
///     agent::{AgentId, AgentInvokeRequest, AgentMessage, Streaming},
/// };
///
/// async fn send_streaming(client: &ZaiClient) -> zai_rs::ZaiResult<()> {
///     let request = AgentInvokeRequest::<Streaming>::builder(AgentId::GeneralTranslation)
///         .message(AgentMessage::user("hello"))
///         .build()?;
///     let _ = request.send_via(client).await?;
///     Ok(())
/// }
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Streaming;

/// Sealed mapping from invocation type-state to its wire-level `stream` value.
pub trait StreamMode: private::Sealed {
    /// Boolean serialized into the request body.
    const WIRE: bool;
}

impl StreamMode for NonStreaming {
    const WIRE: bool = false;
}

impl StreamMode for Streaming {
    const WIRE: bool = true;
}

mod private {
    pub trait Sealed {}
    impl Sealed for super::NonStreaming {}
    impl Sealed for super::Streaming {}
}

/// Builder for an [`AgentInvokeRequest`].
pub struct AgentInvokeRequestBuilder<N: StreamMode> {
    agent_id: AgentId,
    messages: Vec<AgentMessage>,
    custom_variables: AgentCustomVariables,
    marker: PhantomData<N>,
}

impl<N: StreamMode> AgentInvokeRequestBuilder<N> {
    /// Append one message.
    pub fn message(mut self, message: AgentMessage) -> Self {
        self.messages.push(message);
        self
    }

    /// Append multiple messages in order.
    pub fn messages(mut self, messages: impl IntoIterator<Item = AgentMessage>) -> Self {
        self.messages.extend(messages);
        self
    }

    /// Replace the open variables object.
    pub fn custom_variables(mut self, variables: impl Into<AgentCustomVariables>) -> Self {
        self.custom_variables = variables.into();
        self
    }

    /// Validate the required non-empty message list and build the request.
    pub fn build(self) -> ZaiResult<AgentInvokeRequest<N>> {
        if self.messages.is_empty() {
            return Err(crate::client::validation::invalid(
                "messages must contain at least one item",
            ));
        }
        Ok(AgentInvokeRequest {
            agent_id: self.agent_id,
            stream: N::WIRE,
            messages: self.messages,
            custom_variables: self.custom_variables,
            marker: PhantomData,
        })
    }

    /// Switch this request builder to streaming output.
    pub fn streaming(self) -> AgentInvokeRequestBuilder<Streaming> {
        AgentInvokeRequestBuilder {
            agent_id: self.agent_id,
            messages: self.messages,
            custom_variables: self.custom_variables,
            marker: PhantomData,
        }
    }
}

/// Frozen request body for `POST /v1/agents`.
#[derive(Clone, Serialize)]
pub struct AgentInvokeRequest<N: StreamMode> {
    agent_id: AgentId,
    stream: bool,
    messages: Vec<AgentMessage>,
    #[serde(skip_serializing_if = "AgentCustomVariables::is_empty")]
    custom_variables: AgentCustomVariables,
    #[serde(skip)]
    marker: PhantomData<N>,
}

impl<N: StreamMode> std::fmt::Debug for AgentInvokeRequest<N> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let custom_variables = if self.custom_variables.is_empty() {
            "none"
        } else {
            "[REDACTED]"
        };
        formatter
            .debug_struct("AgentInvokeRequest")
            .field("agent_id", &"[REDACTED]")
            .field("stream", &self.stream)
            .field("message_count", &self.messages.len())
            .field("custom_variables", &custom_variables)
            .finish()
    }
}

impl<N: StreamMode> AgentInvokeRequest<N> {
    /// Start an invocation request for a frozen Agent v1 identifier.
    pub fn builder(agent_id: AgentId) -> AgentInvokeRequestBuilder<N> {
        AgentInvokeRequestBuilder {
            agent_id,
            messages: Vec::new(),
            custom_variables: AgentCustomVariables::new(),
            marker: PhantomData,
        }
    }

    /// Return the selected agent identifier.
    pub const fn agent_id(&self) -> AgentId {
        self.agent_id
    }

    /// Return the resolved streaming flag.
    pub const fn stream(&self) -> bool {
        self.stream
    }

    /// Borrow request messages.
    pub fn messages(&self) -> &[AgentMessage] {
        &self.messages
    }

    /// Borrow the open variables object.
    pub const fn custom_variables(&self) -> &AgentCustomVariables {
        &self.custom_variables
    }
}

impl AgentInvokeRequest<NonStreaming> {
    /// Invoke the selected Agent through `client` and decode either a completed
    /// result or an accepted asynchronous invocation.
    pub async fn send_via(&self, client: &ZaiClient) -> ZaiResult<AgentInvokeResponse> {
        let response = client
            .operation(crate::client::routes::AGENTS_INVOKE)
            .send_json::<_, AgentInvokeResponse>(self)
            .await?;
        response.validate()?;
        Ok(response)
    }
}

/// Frozen request body for `POST /v1/agents/async-result`.
#[derive(Clone, Serialize)]
pub struct AgentAsyncResultRequest {
    async_id: String,
    agent_id: String,
}

impl std::fmt::Debug for AgentAsyncResultRequest {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AgentAsyncResultRequest")
            .field("async_id", &"[REDACTED]")
            .field("agent_id", &"[REDACTED]")
            .finish()
    }
}

impl AgentAsyncResultRequest {
    /// Validate the two required identifiers and construct the request.
    pub fn new(agent_id: impl Into<String>, async_id: impl Into<String>) -> ZaiResult<Self> {
        let agent_id = agent_id.into();
        let async_id = async_id.into();
        require_non_blank(&agent_id, "agent_id")?;
        require_non_blank(&async_id, "async_id")?;
        Ok(Self { async_id, agent_id })
    }

    /// Borrow the agent identifier.
    pub fn agent_id(&self) -> &str {
        &self.agent_id
    }

    /// Borrow the asynchronous task identifier.
    pub fn async_id(&self) -> &str {
        &self.async_id
    }

    /// Poll an asynchronous Agent invocation through `client`.
    pub async fn send_via(&self, client: &ZaiClient) -> ZaiResult<AgentAsyncResult> {
        let response = client
            .operation(crate::client::routes::AGENTS_ASYNC_RESULT)
            .send_json::<_, AgentAsyncResult>(self)
            .await?;
        response.validate()?;
        Ok(response)
    }
}

/// One slide page descriptor in conversation custom variables.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentSlidePage {
    #[serde(skip_serializing_if = "Option::is_none")]
    position: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    width: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    height: Option<f64>,
}

impl AgentSlidePage {
    /// Construct a fully specified slide page descriptor.
    pub const fn new(position: f64, width: f64, height: f64) -> Self {
        Self {
            position: Some(position),
            width: Some(width),
            height: Some(height),
        }
    }

    /// Return the slide position.
    pub const fn position(&self) -> Option<f64> {
        self.position
    }

    /// Return the slide width in centimetres.
    pub const fn width(&self) -> Option<f64> {
        self.width
    }

    /// Return the slide height in centimetres.
    pub const fn height(&self) -> Option<f64> {
        self.height
    }
}

/// Closed custom variables accepted by the slide conversation endpoint.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentConversationVariables {
    #[serde(skip_serializing_if = "Option::is_none")]
    include_pdf: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pages: Option<Vec<AgentSlidePage>>,
}

impl AgentConversationVariables {
    /// Construct empty optional conversation variables.
    pub const fn new() -> Self {
        Self {
            include_pdf: None,
            pages: None,
        }
    }

    /// Configure whether a PDF export is included.
    pub const fn with_include_pdf(mut self, include_pdf: bool) -> Self {
        self.include_pdf = Some(include_pdf);
        self
    }

    /// Configure slide page descriptors.
    pub fn with_pages(mut self, pages: impl IntoIterator<Item = AgentSlidePage>) -> Self {
        self.pages = Some(pages.into_iter().collect());
        self
    }

    /// Return the configured PDF flag.
    pub const fn include_pdf(&self) -> Option<bool> {
        self.include_pdf
    }

    /// Borrow configured page descriptors.
    pub fn pages(&self) -> Option<&[AgentSlidePage]> {
        self.pages.as_deref()
    }
}

/// Frozen request body for `POST /v1/agents/conversation`.
#[derive(Clone, Serialize)]
pub struct AgentConversationRequest {
    agent_id: String,
    conversation_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    custom_variables: Option<AgentConversationVariables>,
}

impl std::fmt::Debug for AgentConversationRequest {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AgentConversationRequest")
            .field("agent_id", &"[REDACTED]")
            .field("conversation_id", &"[REDACTED]")
            .field(
                "custom_variables",
                &self.custom_variables.as_ref().map(|_| "[REDACTED]"),
            )
            .finish()
    }
}

impl AgentConversationRequest {
    /// Validate identifiers and construct a conversation-continuation request.
    pub fn new(agent_id: impl Into<String>, conversation_id: impl Into<String>) -> ZaiResult<Self> {
        let agent_id = agent_id.into();
        let conversation_id = conversation_id.into();
        require_non_blank(&agent_id, "agent_id")?;
        require_non_blank(&conversation_id, "conversation_id")?;
        Ok(Self {
            agent_id,
            conversation_id,
            custom_variables: None,
        })
    }

    /// Set the endpoint's closed custom-variables object.
    pub fn with_custom_variables(mut self, variables: AgentConversationVariables) -> Self {
        self.custom_variables = Some(variables);
        self
    }

    /// Borrow the agent identifier.
    pub fn agent_id(&self) -> &str {
        &self.agent_id
    }

    /// Borrow the conversation identifier.
    pub fn conversation_id(&self) -> &str {
        &self.conversation_id
    }

    /// Borrow the optional custom variables.
    pub const fn custom_variables(&self) -> Option<&AgentConversationVariables> {
        self.custom_variables.as_ref()
    }

    /// Continue an Agent conversation through `client`.
    pub async fn send_via(&self, client: &ZaiClient) -> ZaiResult<AgentConversationResponse> {
        let response = client
            .operation(crate::client::routes::AGENTS_CONVERSATION)
            .send_json::<_, AgentConversationResponse>(self)
            .await?;
        response.validate()?;
        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invoke_request_serializes_the_exact_nonstreaming_schema() {
        let request = AgentInvokeRequest::<NonStreaming>::builder(AgentId::GeneralTranslation)
            .message(AgentMessage::user("hello"))
            .build()
            .unwrap();

        assert_eq!(
            serde_json::to_value(&request).unwrap(),
            serde_json::json!({
                "agent_id": "general_translation",
                "stream": false,
                "messages": [{"role": "user", "content": "hello"}]
            })
        );
        assert!(!format!("{request:?}").contains("hello"));
    }

    #[test]
    fn invoke_request_serializes_closed_multimodal_parts_and_open_variables() {
        let mut variables = AgentCustomVariables::new();
        variables.insert("target_lang", serde_json::json!("en"));
        let request = AgentInvokeRequest::<NonStreaming>::builder(AgentId::GeneralTranslation)
            .message(AgentMessage::user(vec![
                AgentRequestContentPart::text("translate this"),
                AgentRequestContentPart::file_id("file-1"),
                AgentRequestContentPart::file_url("https://example.test/report.pdf"),
                AgentRequestContentPart::image_url("https://example.test/image.png"),
            ]))
            .custom_variables(variables)
            .streaming()
            .build()
            .unwrap();

        assert_eq!(
            serde_json::to_value(request).unwrap(),
            serde_json::json!({
                "agent_id": "general_translation",
                "stream": true,
                "messages": [{
                    "role": "user",
                    "content": [
                        {"type": "text", "text": "translate this"},
                        {"type": "file_id", "file_id": "file-1"},
                        {"type": "file_url", "file_url": "https://example.test/report.pdf"},
                        {"type": "image_url", "image_url": "https://example.test/image.png"}
                    ]
                }],
                "custom_variables": {"target_lang": "en"}
            })
        );
    }

    #[test]
    fn invoke_request_requires_at_least_one_message() {
        assert!(
            AgentInvokeRequest::<NonStreaming>::builder(AgentId::GeneralTranslation)
                .build()
                .is_err()
        );
    }

    #[test]
    fn async_result_request_is_exact_and_redacted() {
        assert!(AgentAsyncResultRequest::new(" ", "task-1").is_err());
        assert!(AgentAsyncResultRequest::new("agent-1", " ").is_err());
        let request = AgentAsyncResultRequest::new("agent-1", "task-1").unwrap();
        assert_eq!(
            serde_json::to_value(&request).unwrap(),
            serde_json::json!({"async_id": "task-1", "agent_id": "agent-1"})
        );
        let debug = format!("{request:?}");
        assert!(!debug.contains("agent-1"));
        assert!(!debug.contains("task-1"));
    }

    #[test]
    fn conversation_request_uses_custom_variables_instead_of_messages() {
        let variables = AgentConversationVariables::new()
            .with_include_pdf(true)
            .with_pages([AgentSlidePage::new(1.0, 25.4, 14.29)]);
        let request = AgentConversationRequest::new("slides_glm_agent", "conversation-1")
            .unwrap()
            .with_custom_variables(variables);

        assert_eq!(
            serde_json::to_value(&request).unwrap(),
            serde_json::json!({
                "agent_id": "slides_glm_agent",
                "conversation_id": "conversation-1",
                "custom_variables": {
                    "include_pdf": true,
                    "pages": [{"position": 1.0, "width": 25.4, "height": 14.29}]
                }
            })
        );
        let debug = format!("{request:?}");
        assert!(!debug.contains("slides_glm_agent"));
        assert!(!debug.contains("conversation-1"));
    }
}
