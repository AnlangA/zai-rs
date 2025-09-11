//! # Chat Completion Data Models
//!
//! This module defines the core data structures for chat completion requests,
//! implementing type-safe chat interactions with the Zhipu AI API.
//!
//! ## Type-State Pattern
//!
//! The implementation uses Rust's type system to enforce compile-time guarantees
//! about streaming capabilities through phantom types (`StreamOn`/`StreamOff`).
//!
//! ## Features
//!
//! - **Type-safe model binding** - Compile-time verification of model-message compatibility
//! - **Builder pattern** - Fluent API for request construction
//! - **Streaming support** - Type-state based streaming capability enforcement
//! - **Tool integration** - Support for function calling and tool usage
//! - **Parameter control** - Temperature, top-p, max tokens, and other generation parameters

use super::super::chat_base_request::*;
use super::super::tools::*;
use super::super::traits::*;
use crate::client::http::HttpClient;
use serde::Serialize;
use std::marker::PhantomData;

// Type-state is defined in model::traits::{StreamState, StreamOn, StreamOff}

/// Type-safe chat completion request structure.
///
/// This struct represents a chat completion request with compile-time guarantees
/// for model compatibility and streaming capabilities.
///
/// ## Type Parameters
///
/// - `N` - The AI model type (must implement `ModelName + Chat`)
/// - `M` - The message type (must form a valid bound with the model)
/// - `S` - Stream state (`StreamOn` or `StreamOff`, defaults to `StreamOff`)
///
/// ## Examples
///
/// ```rust,ignore
/// let model = GLM4_5_flash {};
/// let messages = TextMessage::user("Hello, how are you?");
/// let request = ChatCompletion::new(model, messages, api_key);
/// ```
pub struct ChatCompletion<N, M, S = StreamOff>
where
    N: ModelName + Chat,
    (N, M): Bounded,
    ChatBody<N, M>: Serialize,
    S: StreamState,
{
    /// API key for authentication with the Zhipu AI service.
    pub key: String,

    /// The request body containing model, messages, and parameters.
    body: ChatBody<N, M>,

    /// Phantom data to track streaming capability at compile time.
    _stream: PhantomData<S>,
}

impl<N, M> ChatCompletion<N, M, StreamOff>
where
    N: ModelName + Chat,
    (N, M): Bounded,
    ChatBody<N, M>: Serialize,
{
    /// Creates a new non-streaming chat completion request.
    ///
    /// ## Arguments
    ///
    /// * `model` - The AI model to use for completion
    /// * `messages` - The conversation messages
    /// * `key` - API key for authentication
    ///
    /// ## Returns
    ///
    /// A new `ChatCompletion` instance configured for non-streaming requests.
    pub fn new(model: N, messages: M, key: String) -> ChatCompletion<N, M, StreamOff> {
        let body = ChatBody::new(model, messages);
        ChatCompletion {
            body,
            key,
            _stream: PhantomData,
        }
    }

    /// Gets mutable access to the request body for further customization.
    ///
    /// This method allows modification of request parameters after initial creation.
    pub fn body_mut(&mut self) -> &mut ChatBody<N, M> {
        &mut self.body
    }

    /// Adds additional messages to the conversation.
    ///
    /// This method provides a fluent interface for building conversation context.
    ///
    /// ## Arguments
    ///
    /// * `messages` - Additional messages to append to the conversation
    ///
    /// ## Returns
    ///
    /// Self with the updated message collection, enabling method chaining.
    pub fn add_messages(mut self, messages: M) -> Self {
        self.body = self.body.add_messages(messages);
        self
    }
    pub fn with_request_id(mut self, request_id: impl Into<String>) -> Self {
        self.body = self.body.with_request_id(request_id);
        self
    }
    pub fn with_do_sample(mut self, do_sample: bool) -> Self {
        self.body = self.body.with_do_sample(do_sample);
        self
    }
    #[deprecated(note = "Use enable_stream()/disable_stream() for compile-time guarantees")]
    pub fn with_stream(mut self, stream: bool) -> Self {
        self.body = self.body.with_stream(stream);
        self
    }
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.body = self.body.with_temperature(temperature);
        self
    }
    pub fn with_top_p(mut self, top_p: f32) -> Self {
        self.body = self.body.with_top_p(top_p);
        self
    }
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.body = self.body.with_max_tokens(max_tokens);
        self
    }
    pub fn add_tool(mut self, tool: Tools) -> Self {
        self.body = self.body.add_tools(tool);
        self
    }
    pub fn add_tools(mut self, tools: Vec<Tools>) -> Self {
        self.body = self.body.extend_tools(tools);
        self
    }
    pub fn with_user_id(mut self, user_id: impl Into<String>) -> Self {
        self.body = self.body.with_user_id(user_id);
        self
    }
    pub fn with_stop(mut self, stop: String) -> Self {
        self.body = self.body.with_stop(stop);
        self
    }

    // Optional: only available when model supports thinking
    pub fn with_thinking(mut self, thinking: ThinkingType) -> Self
    where
        N: ThinkEnable,
    {
        self.body = self.body.with_thinking(thinking);
        self
    }

    /// Enables streaming for this chat completion request.
    ///
    /// This method transitions the request to streaming mode, allowing
    /// real-time response processing through Server-Sent Events (SSE).
    ///
    /// ## Returns
    ///
    /// A new `ChatCompletion` instance with streaming enabled (`StreamOn`).
    pub fn enable_stream(mut self) -> ChatCompletion<N, M, StreamOn> {
        self.body.stream = Some(true);
        ChatCompletion {
            key: self.key,
            body: self.body,
            _stream: PhantomData,
        }
    }

    /// Disables streaming for this chat completion request.
    ///
    /// This method ensures the request will receive a complete response
    /// rather than streaming chunks.
    ///
    /// ## Returns
    ///
    /// A new `ChatCompletion` instance with streaming disabled (`StreamOff`).
    pub fn disable_stream(mut self) -> ChatCompletion<N, M, StreamOff> {
        self.body.stream = Some(false);
        ChatCompletion {
            key: self.key,
            body: self.body,
            _stream: PhantomData,
        }
    }
}

impl<N, M, S> HttpClient for ChatCompletion<N, M, S>
where
    N: ModelName + Serialize + Chat,
    M: Serialize,
    (N, M): Bounded,
    S: StreamState,
{
    type Body = ChatBody<N, M>;
    type ApiUrl = &'static str;
    type ApiKey = String;

    /// Returns the Zhipu AI chat completions API endpoint URL.
    fn api_url(&self) -> &Self::ApiUrl {
        &"https://open.bigmodel.cn/api/paas/v4/chat/completions"
    }
    fn api_key(&self) -> &Self::ApiKey {
        &self.key
    }
    fn body(&self) -> &Self::Body {
        &self.body
    }
}

/// Enables Server-Sent Events (SSE) streaming for streaming-enabled chat completions.
///
/// This implementation allows streaming chat completions to be processed
/// incrementally as responses arrive from the API.
impl<N, M> crate::model::traits::SseStreamable for ChatCompletion<N, M, StreamOn>
where
    N: ModelName + Serialize + Chat,
    M: Serialize,
    (N, M): Bounded,
{
}

/// Provides streaming extension methods for streaming-enabled chat completions.
///
/// This implementation enables the use of streaming-specific methods
/// for processing chat responses in real-time.
impl<N, M> crate::model::stream_ext::StreamChatLikeExt for ChatCompletion<N, M, StreamOn>
where
    N: ModelName + Serialize + Chat,
    M: Serialize,
    (N, M): Bounded,
{
}
