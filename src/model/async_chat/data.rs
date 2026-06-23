use std::{marker::PhantomData, sync::Arc};

use serde::Serialize;
use validator::Validate;

use super::super::{chat_base_request::*, tools::*, traits::*};
use crate::client::{
    endpoints::{ApiBase, EndpointConfig, paths},
    http::{HttpClient, HttpClientConfig, parse_typed_response},
};

/// Asynchronous (queued) chat-completion request builder.
///
/// Generic over the model `N`, the message type `M`, and a stream type-state
/// `S` (`StreamOff` by default, `StreamOn` after
/// [`enable_stream`](Self::enable_stream)). Posts to the
/// `async/chat/completions` endpoint and returns a task id that must be polled
/// via [`AsyncChatGetRequest`](crate::model::async_chat_get::data::AsyncChatGetRequest).
pub struct AsyncChatCompletion<N, M, S = StreamOff>
where
    N: ModelName + AsyncChat,
    (N, M): Bounded,
    ChatBody<N, M>: Serialize,
    S: StreamState,
{
    /// Zhipu AI API key used for `Authorization: Bearer …`.
    pub key: String,
    url: String,
    endpoint_config: EndpointConfig,
    api_base: ApiBase,
    http_config: Arc<HttpClientConfig>,
    body: ChatBody<N, M>,
    _stream: PhantomData<S>,
}

impl<N, M> AsyncChatCompletion<N, M, StreamOff>
where
    N: ModelName + AsyncChat,
    (N, M): Bounded,
    ChatBody<N, M>: Serialize,
{
    /// Create a new async chat request from a model, the first message batch,
    /// and an API key.
    pub fn new(model: N, messages: M, key: String) -> Self {
        let body = ChatBody::new(model, messages);
        let endpoint_config = EndpointConfig::default();
        let api_base = ApiBase::PaasV4;
        let url = endpoint_config.url(&api_base, paths::ASYNC_CHAT_COMPLETIONS);
        Self {
            body,
            key,
            url,
            endpoint_config,
            api_base,
            http_config: Arc::new(HttpClientConfig::default()),
            _stream: PhantomData,
        }
    }

    /// Borrow the underlying `ChatBody` mutably for advanced tweaks.
    pub fn body_mut(&mut self) -> &mut ChatBody<N, M> {
        &mut self.body
    }

    // Fluent, builder-style forwarding methods to mutate inner ChatBody and return
    // Self
    /// Append another message batch to the conversation.
    pub fn add_messages(mut self, messages: M) -> Self {
        self.body = self.body.add_messages(messages);
        self
    }
    /// Set the client-side request id.
    pub fn with_request_id(mut self, request_id: impl Into<String>) -> Self {
        self.body = self.body.with_request_id(request_id);
        self
    }
    /// Enable/disable sampling (`do_sample`).
    pub fn with_do_sample(mut self, do_sample: bool) -> Self {
        self.body = self.body.with_do_sample(do_sample);
        self
    }
    #[deprecated(note = "Use enable_stream()/disable_stream() for compile-time guarantees")]
    /// Deprecated: prefer [`enable_stream`](Self::enable_stream) /
    /// [`disable_stream`](Self::disable_stream) for compile-time guarantees.
    pub fn with_stream(mut self, stream: bool) -> Self {
        self.body = self.body.with_stream(stream);
        self
    }
    /// Enable/disable tool-call streaming (requires a model that supports it).
    pub fn with_tool_stream(mut self, tool_stream: bool) -> Self
    where
        N: ToolStreamEnable,
    {
        self.body = self.body.with_tool_stream(tool_stream);
        self
    }

    /// Set the sampling temperature.
    pub fn with_temperature(mut self, temperature: f64) -> Self {
        self.body = self.body.with_temperature(temperature);
        self
    }
    /// Set the nucleus-sampling probability (`top_p`).
    pub fn with_top_p(mut self, top_p: f64) -> Self {
        self.body = self.body.with_top_p(top_p);
        self
    }
    /// Set the maximum number of tokens to generate.
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.body = self.body.with_max_tokens(max_tokens);
        self
    }
    /// Add a single tool to the request.
    pub fn add_tool(mut self, tool: Tools) -> Self {
        self.body = self.body.add_tools(tool);
        self
    }
    /// Add multiple tools to the request at once.
    pub fn add_tools(mut self, tools: Vec<Tools>) -> Self {
        self.body = self.body.extend_tools(tools);
        self
    }
    /// Set the end-user id (used for abuse monitoring).
    pub fn with_user_id(mut self, user_id: impl Into<String>) -> Self {
        self.body = self.body.with_user_id(user_id);
        self
    }
    /// Add a stop sequence that halts generation when encountered.
    pub fn with_stop(mut self, stop: String) -> Self {
        self.body = self.body.with_stop(stop);
        self
    }

    /// Override the base URL (uses [`ApiBase::Custom`]).
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.api_base = ApiBase::Custom(base_url.into());
        self.url = self
            .endpoint_config
            .url(&self.api_base, paths::ASYNC_CHAT_COMPLETIONS);
        self
    }

    /// Replace the full [`EndpointConfig`] used to resolve URLs.
    pub fn with_endpoint_config(mut self, endpoint_config: EndpointConfig) -> Self {
        self.endpoint_config = endpoint_config;
        self.url = self
            .endpoint_config
            .url(&self.api_base, paths::ASYNC_CHAT_COMPLETIONS);
        self
    }

    /// Replace the HTTP client configuration (timeouts, retries, …).
    pub fn with_http_config(mut self, config: HttpClientConfig) -> Self {
        self.http_config = Arc::new(config);
        self
    }

    // Optional: only available when model supports thinking
    /// Enable thinking mode (requires a model that supports it).
    pub fn with_thinking(mut self, thinking: ThinkingType) -> Self
    where
        N: ThinkEnable,
    {
        self.body = self.body.with_thinking(thinking);
        self
    }

    // Optional: only available for GLM-5.2+ (reasoning_effort support)
    /// Set the reasoning effort (GLM-5.2+ only).
    pub fn with_reasoning_effort(mut self, effort: ReasoningEffort) -> Self
    where
        N: ReasoningEffortEnable,
    {
        self.body = self.body.with_reasoning_effort(effort);
        self
    }

    // Type-state toggles
    /// Switch this builder into streaming mode (consumes `self`).
    pub fn enable_stream(mut self) -> AsyncChatCompletion<N, M, StreamOn> {
        self.body.stream = Some(true);
        AsyncChatCompletion {
            key: self.key,
            url: self.url,
            endpoint_config: self.endpoint_config,
            api_base: self.api_base,
            http_config: self.http_config,
            body: self.body,
            _stream: PhantomData,
        }
    }

    /// Validate request parameters for non-stream async chat (StreamOff)
    pub fn validate(&self) -> crate::ZaiResult<()> {
        self.body
            .validate()
            .map_err(crate::client::error::ZaiError::from)?;
        if matches!(self.body.stream, Some(true)) {
            return Err(crate::client::error::ZaiError::ApiError {
                code: 1200,
                message: "stream=true detected; use enable_stream() and streaming APIs instead"
                    .to_string(),
            });
        }

        Ok(())
    }

    /// Submit the request and await the (non-streaming) response.
    pub async fn send(
        &self,
    ) -> crate::ZaiResult<crate::model::chat_base_response::ChatCompletionResponse>
    where
        N: serde::Serialize,
        M: serde::Serialize,
    {
        self.validate()?;

        let resp: reqwest::Response = self.post().await?;

        let parsed =
            parse_typed_response::<crate::model::chat_base_response::ChatCompletionResponse>(resp)
                .await?;
        Ok(parsed)
    }
}

impl<N, M> AsyncChatCompletion<N, M, StreamOn>
where
    N: ModelName + AsyncChat,
    (N, M): Bounded,
    ChatBody<N, M>: Serialize,
{
    /// Enable/disable tool-call streaming (requires a model that supports it).
    pub fn with_tool_stream(mut self, tool_stream: bool) -> Self
    where
        N: ToolStreamEnable,
    {
        self.body = self.body.with_tool_stream(tool_stream);
        self
    }

    /// Switch this builder back into non-streaming mode (consumes `self`).
    pub fn disable_stream(mut self) -> AsyncChatCompletion<N, M, StreamOff> {
        self.body.stream = Some(false);
        // Reset tool_stream when disabling streaming since tool_stream depends on
        // stream
        self.body.tool_stream = None;
        AsyncChatCompletion {
            key: self.key,
            url: self.url,
            endpoint_config: self.endpoint_config,
            api_base: self.api_base,
            http_config: self.http_config,
            body: self.body,
            _stream: PhantomData,
        }
    }
}

impl<N, M, S> HttpClient for AsyncChatCompletion<N, M, S>
where
    N: ModelName + Serialize + AsyncChat,
    M: Serialize,
    (N, M): Bounded,
    S: StreamState,
{
    type Body = ChatBody<N, M>;
    type ApiUrl = String;
    type ApiKey = String;

    fn api_url(&self) -> &Self::ApiUrl {
        &self.url
    }
    fn api_key(&self) -> &Self::ApiKey {
        &self.key
    }
    fn body(&self) -> &Self::Body {
        &self.body
    }

    fn http_config(&self) -> Arc<HttpClientConfig> {
        self.http_config.clone()
    }
}

impl<N, M> crate::model::traits::SseStreamable for AsyncChatCompletion<N, M, StreamOn>
where
    N: ModelName + Serialize + AsyncChat,
    M: Serialize,
    (N, M): Bounded,
{
}

// Enable typed streaming extension methods for AsyncChatCompletion<...,
// StreamOn>
impl<N, M> crate::model::stream_ext::StreamChatLikeExt for AsyncChatCompletion<N, M, StreamOn>
where
    N: ModelName + Serialize + AsyncChat,
    M: Serialize,
    (N, M): Bounded,
{
}
