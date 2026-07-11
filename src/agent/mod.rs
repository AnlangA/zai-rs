//! # Agent v1 API (plan P04).
//!
//! The previous 0.4 agents CRUD + chat + history surface (the legacy PAAS v4
//! agents path) is REMOVED — it targeted a non-existent endpoint family. This
//! module implements the official Agent v1 contract (plan §13.2) with three
//! operations:
//!
//! - `POST /v1/agents` — `invoke` (non-stream) / `stream` (SSE), shared request
//!   via type-state.
//! - `POST /v1/agents/async-result` — `async_result`.
//! - `POST /v1/agents/conversation` — `conversation`.
//!
//! No compatibility alias is kept for the removed 0.4 surface.

pub mod request;
pub mod response;

pub use request::*;
pub use response::*;

use std::marker::PhantomData;

use serde::Serialize;

use crate::ZaiResult;
use crate::client::error::codes;

// ---------------------------------------------------------------------------
// Type-state markers for streaming vs non-streaming invoke.
// ---------------------------------------------------------------------------

/// Type-state marker: a non-streaming agent invocation.
#[derive(Debug, Clone, Copy, Default)]
pub struct NonStreaming;
/// Type-state marker: a streaming agent invocation (serializes `stream=true`).
#[derive(Debug, Clone, Copy, Default)]
pub struct Streaming;

/// Trait bound linking a stream type-state to its `stream` wire value.
pub trait StreamMode: private::Sealed {
    /// The `stream` value serialized into the request body (`None` omits it).
    const WIRE: Option<bool>;
}
impl StreamMode for NonStreaming {
    const WIRE: Option<bool> = Some(false);
}
impl StreamMode for Streaming {
    const WIRE: Option<bool> = Some(true);
}

mod private {
    pub trait Sealed {}
    impl Sealed for super::NonStreaming {}
    impl Sealed for super::Streaming {}
}

// ---------------------------------------------------------------------------
// Public request builders.
// ---------------------------------------------------------------------------

impl<N: StreamMode> AgentInvokeRequest<N> {
    /// Build an invoke request for `agent_id` with at least one message.
    pub fn builder(agent_id: impl Into<String>) -> AgentInvokeRequestBuilder<N> {
        AgentInvokeRequestBuilder {
            agent_id: agent_id.into(),
            messages: Vec::new(),
            custom_variables: serde_json::Map::new(),
            _marker: PhantomData,
        }
    }
}

/// Builder for [`AgentInvokeRequest`].
pub struct AgentInvokeRequestBuilder<N: StreamMode> {
    agent_id: String,
    messages: Vec<AgentMessage>,
    custom_variables: serde_json::Map<String, serde_json::Value>,
    _marker: PhantomData<N>,
}

impl<N: StreamMode> AgentInvokeRequestBuilder<N> {
    /// Add a message (role must be system/user/assistant).
    pub fn message(mut self, msg: AgentMessage) -> Self {
        self.messages.push(msg);
        self
    }
    /// Set the open `custom_variables` map (plan §13.1: additionalProperties).
    pub fn custom_variables(mut self, vars: serde_json::Map<String, serde_json::Value>) -> Self {
        self.custom_variables = vars;
        self
    }
    /// Finalize, validating `agent_id` non-empty and `messages` non-empty.
    pub fn build(self) -> ZaiResult<AgentInvokeRequest<N>> {
        if self.agent_id.trim().is_empty() {
            return Err(crate::ZaiError::ApiError {
                code: codes::SDK_VALIDATION,
                message: "agent_id must be non-empty".to_string(),
            });
        }
        if self.messages.is_empty() {
            return Err(crate::ZaiError::ApiError {
                code: codes::SDK_VALIDATION,
                message: "AgentInvokeRequest requires at least one message".to_string(),
            });
        }
        // Validate message roles.
        for m in &self.messages {
            if !matches!(m.role.as_str(), "system" | "user" | "assistant") {
                return Err(crate::ZaiError::ApiError {
                    code: codes::SDK_VALIDATION,
                    message: format!("invalid message role `{}`", m.role),
                });
            }
        }
        Ok(AgentInvokeRequest {
            agent_id: self.agent_id,
            messages: self.messages,
            custom_variables: self.custom_variables,
            _marker: PhantomData,
            stream: N::WIRE,
        })
    }

    /// Switch this builder into streaming mode (consumes `self`).
    pub fn streaming(self) -> AgentInvokeRequestBuilder<Streaming> {
        AgentInvokeRequestBuilder {
            agent_id: self.agent_id,
            messages: self.messages,
            custom_variables: self.custom_variables,
            _marker: PhantomData,
        }
    }
}

/// `POST /v1/agents` request body (plan §13.2).
#[derive(Debug, Clone, Serialize)]
pub struct AgentInvokeRequest<N: StreamMode> {
    pub agent_id: String,
    pub messages: Vec<AgentMessage>,
    #[serde(skip_serializing_if = "serde_json::Map::is_empty")]
    pub custom_variables: serde_json::Map<String, serde_json::Value>,
    /// Serialized from the type-state; `true` for streaming, `false` otherwise.
    /// The `PhantomData<N>` is NOT serialized — the resolved value is emitted
    /// via the `stream` field below.
    #[serde(skip)]
    pub _marker: PhantomData<N>,
    /// The `stream` wire value, derived from the type-state at construction.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
}

impl<N: StreamMode> AgentInvokeRequest<N> {
    /// The resolved `stream` wire value.
    pub fn stream_value(&self) -> Option<bool> {
        self.stream
    }
}

/// `POST /v1/agents/async-result` request body.
#[derive(Debug, Clone, Serialize)]
pub struct AgentAsyncResultRequest {
    pub agent_id: String,
    pub async_id: String,
}

impl AgentAsyncResultRequest {
    pub fn builder(
        agent_id: impl Into<String>,
        async_id: impl Into<String>,
    ) -> AgentAsyncResultRequestBuilder {
        AgentAsyncResultRequestBuilder {
            agent_id: agent_id.into(),
            async_id: async_id.into(),
        }
    }
}

pub struct AgentAsyncResultRequestBuilder {
    agent_id: String,
    async_id: String,
}

impl AgentAsyncResultRequestBuilder {
    pub fn build(self) -> ZaiResult<AgentAsyncResultRequest> {
        if self.agent_id.trim().is_empty() || self.async_id.trim().is_empty() {
            return Err(crate::ZaiError::ApiError {
                code: codes::SDK_VALIDATION,
                message: "agent_id and async_id must both be non-empty".to_string(),
            });
        }
        Ok(AgentAsyncResultRequest {
            agent_id: self.agent_id,
            async_id: self.async_id,
        })
    }
}

/// `POST /v1/agents/conversation` request body.
#[derive(Debug, Clone, Serialize)]
pub struct AgentConversationRequest {
    pub agent_id: String,
    pub conversation_id: String,
    pub messages: Vec<AgentMessage>,
}

impl AgentConversationRequest {
    pub fn builder(
        agent_id: impl Into<String>,
        conversation_id: impl Into<String>,
    ) -> AgentConversationRequestBuilder {
        AgentConversationRequestBuilder {
            agent_id: agent_id.into(),
            conversation_id: conversation_id.into(),
            messages: Vec::new(),
        }
    }
}

pub struct AgentConversationRequestBuilder {
    agent_id: String,
    conversation_id: String,
    messages: Vec<AgentMessage>,
}

impl AgentConversationRequestBuilder {
    pub fn message(mut self, msg: AgentMessage) -> Self {
        self.messages.push(msg);
        self
    }
    pub fn build(self) -> ZaiResult<AgentConversationRequest> {
        if self.agent_id.trim().is_empty() || self.conversation_id.trim().is_empty() {
            return Err(crate::ZaiError::ApiError {
                code: codes::SDK_VALIDATION,
                message: "agent_id and conversation_id must both be non-empty".to_string(),
            });
        }
        if self.messages.is_empty() {
            return Err(crate::ZaiError::ApiError {
                code: codes::SDK_VALIDATION,
                message: "AgentConversationRequest requires at least one message".to_string(),
            });
        }
        Ok(AgentConversationRequest {
            agent_id: self.agent_id,
            conversation_id: self.conversation_id,
            messages: self.messages,
        })
    }
}

/// Construct a single message.
pub fn message(role: &str, content: impl Into<AgentContent>) -> AgentMessage {
    AgentMessage {
        role: role.to_string(),
        content: content.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invoke_request_serializes_stream_false_for_nonstreaming() {
        let req = AgentInvokeRequest::<NonStreaming>::builder("agent-1")
            .message(message("user", "hi"))
            .build()
            .unwrap();
        assert_eq!(req.stream_value(), Some(false));
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["stream"], false);
        assert_eq!(json["agent_id"], "agent-1");
    }

    #[test]
    fn invoke_request_serializes_stream_true_for_streaming() {
        let req = AgentInvokeRequest::<NonStreaming>::builder("agent-1")
            .message(message("user", "hi"))
            .streaming()
            .build()
            .unwrap();
        assert_eq!(req.stream_value(), Some(true));
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["stream"], true);
    }

    #[test]
    fn invoke_rejects_empty_agent_id_and_no_messages() {
        assert!(
            AgentInvokeRequest::<NonStreaming>::builder("")
                .message(message("user", "hi"))
                .build()
                .is_err()
        );
        assert!(
            AgentInvokeRequest::<NonStreaming>::builder("a")
                .build()
                .is_err()
        );
    }

    #[test]
    fn invoke_rejects_invalid_role() {
        assert!(
            AgentInvokeRequest::<NonStreaming>::builder("a")
                .message(message("tool", "hi"))
                .build()
                .is_err()
        );
    }

    #[test]
    fn async_result_rejects_blank_ids() {
        assert!(AgentAsyncResultRequest::builder("", "x").build().is_err());
        assert!(AgentAsyncResultRequest::builder("x", "").build().is_err());
        assert!(AgentAsyncResultRequest::builder("a", "b").build().is_ok());
    }

    #[test]
    fn conversation_rejects_blank_or_no_messages() {
        assert!(
            AgentConversationRequest::builder("", "c")
                .message(message("user", "hi"))
                .build()
                .is_err()
        );
        assert!(AgentConversationRequest::builder("a", "c").build().is_err());
        assert!(
            AgentConversationRequest::builder("a", "c")
                .message(message("user", "hi"))
                .build()
                .is_ok()
        );
    }
}
