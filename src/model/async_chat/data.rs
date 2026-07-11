use std::sync::Arc;

use serde::Serialize;
use validator::Validate;

use super::super::{chat_base_request::*, tools::*, traits::*};
use crate::client::{
    endpoints::{ApiBase, EndpointConfig, paths},
    http::{HttpClient, HttpClientConfig, parse_typed_response},
};

/// Asynchronous (queued) chat-completion request builder.
///
/// Posts to the `async/chat/completions` task-submission endpoint and returns a
/// task id that must be polled via
/// [`AsyncChatGetRequest`](crate::model::async_chat_get::data::AsyncChatGetRequest).
///
/// **P04.5**: the official async-chat endpoint is a *task submission* interface
/// and does NOT accept `stream`. The 0.4 `stream` type-state (`StreamOn`/
/// `enable_stream`/`with_stream`/`SseStreamable`) is removed; the request body
/// never serializes `stream`. (Plan §2.2.3 / P04.6.)
pub struct AsyncChatCompletion<N, M>
where
    N: ModelName + AsyncChat,
    (N, M): Bounded,
    ChatBody<N, M>: Serialize,
{
    /// Zhipu AI API key used for `Authorization: Bearer …`.
    pub key: String,
    url: String,
    endpoint_config: EndpointConfig,
    api_base: ApiBase,
    http_config: Arc<HttpClientConfig>,
    body: ChatBody<N, M>,
}

impl<N, M> AsyncChatCompletion<N, M>
where
    N: ModelName + AsyncChat,
    (N, M): Bounded,
    ChatBody<N, M>: Serialize,
{
    /// Create a new async chat request from a model, the first message batch,
    /// and an API key.
    pub fn new(model: N, messages: M, key: impl Into<String>) -> Self {
        let body = ChatBody::new(model, messages);
        let endpoint_config = EndpointConfig::default();
        let api_base = ApiBase::PaasV4;
        let url = endpoint_config.url(&api_base, paths::ASYNC_CHAT_COMPLETIONS);
        Self {
            body,
            key: key.into(),
            url,
            endpoint_config,
            api_base,
            http_config: Arc::new(HttpClientConfig::default()),
        }
    }

    /// Borrow the underlying `ChatBody` mutably for advanced tweaks.
    pub fn body_mut(&mut self) -> &mut ChatBody<N, M> {
        &mut self.body
    }

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
        self.body = self.body.add_tool(tool);
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

    /// Enable thinking mode (requires a model that supports it).
    pub fn with_thinking(mut self, thinking: ThinkingType) -> Self
    where
        N: ThinkEnable,
    {
        self.body = self.body.with_thinking(thinking);
        self
    }

    /// Set the reasoning effort (GLM-5.2+ only).
    pub fn with_reasoning_effort(mut self, effort: ReasoningEffort) -> Self
    where
        N: ReasoningEffortEnable,
    {
        self.body = self.body.with_reasoning_effort(effort);
        self
    }

    /// Validate request parameters. The async-chat endpoint does not accept
    /// `stream`; a body carrying `stream=true` is rejected here.
    pub fn validate(&self) -> crate::ZaiResult<()> {
        self.body
            .validate()
            .map_err(crate::client::error::ZaiError::from)?;
        if matches!(self.body.stream, Some(true)) {
            return Err(crate::client::error::ZaiError::ApiError {
                code: crate::client::error::codes::SDK_VALIDATION,
                message: "async chat is a task-submission endpoint and does not accept stream"
                    .to_string(),
            });
        }
        Ok(())
    }

    /// Submit the request and await the (non-streaming) task-creation response.
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

impl<N, M> HttpClient for AsyncChatCompletion<N, M>
where
    N: ModelName + Serialize + AsyncChat,
    M: Serialize,
    (N, M): Bounded,
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
