use std::marker::PhantomData;
use std::sync::Arc;

use serde::Serialize;
use validator::Validate;

use super::super::{
    chat_base_request::*, chat_base_response::ChatCompletionResponse, tools::*, traits::*,
};
use crate::client::ZaiClient;
use crate::client::http::{HttpClientConfig, parse_typed_response, send_json_request};

/// Chat-completion request builder (plan P05: migrated to route through
/// [`ZaiClient`]).
///
/// Generic over the model `N`, the message type `M`, and a stream type-state
/// `S` (`StreamOff` by default, `StreamOn` after
/// [`enable_stream`](Self::enable_stream)).
///
/// **P05**: the `key`/`url`/`endpoint_config`/`api_base`/`http_config` fields
/// and the `with_base_url`/`with_endpoint_config`/`with_http_config`/`with_url`
/// methods are REMOVED. Credentials and transport live on the `ZaiClient`,
/// passed to `send`/`send_via` on the client.
pub struct ChatCompletion<N, M, S = StreamOff>
where
    N: ModelName + Chat,
    (N, M): Bounded,
    ChatBody<N, M>: Serialize,
    S: StreamState,
{
    body: ChatBody<N, M>,
    _stream: PhantomData<S>,
}

impl<N, M> ChatCompletion<N, M, StreamOff>
where
    N: ModelName + Chat,
    (N, M): Bounded,
    ChatBody<N, M>: Serialize,
{
    /// Create a new chat request from a model and the first message batch.
    ///
    /// **P05**: no longer takes an API key — the key is provided by the
    /// [`ZaiClient`] at send time.
    pub fn new(model: N, messages: M) -> ChatCompletion<N, M, StreamOff> {
        let body = ChatBody::new(model, messages);
        ChatCompletion {
            body,
            _stream: PhantomData,
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
    /// Set the tool choice.
    pub fn with_tool_choice(mut self, tool_choice: ToolChoice) -> Self {
        self.body = self.body.with_tool_choice(tool_choice);
        self
    }
    /// Set the response format.
    pub fn with_response_format(mut self, format: ResponseFormat) -> Self {
        self.body = self.body.with_response_format(format);
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

    /// Enables streaming for this chat completion request.
    pub fn enable_stream(mut self) -> ChatCompletion<N, M, StreamOn> {
        self.body.stream = Some(true);
        ChatCompletion {
            body: self.body,
            _stream: PhantomData,
        }
    }

    /// Validate request parameters for non-stream chat (StreamOff)
    pub fn validate(&self) -> crate::ZaiResult<()> {
        self.body
            .validate()
            .map_err(crate::client::error::ZaiError::from)?;
        if matches!(self.body.stream, Some(true)) {
            return Err(crate::client::error::ZaiError::ApiError {
                code: crate::client::error::codes::SDK_VALIDATION,
                message: "stream=true detected; use enable_stream() and streaming APIs instead"
                    .to_string(),
            });
        }
        Ok(())
    }

    /// Submit the request via a [`ZaiClient`] and await the (non-streaming)
    /// response.
    ///
    /// **P05**: credentials and transport come from the client; the request
    /// type carries no key/config of its own.
    pub async fn send_via(&self, client: &ZaiClient) -> crate::ZaiResult<ChatCompletionResponse>
    where
        N: serde::Serialize,
        M: serde::Serialize,
    {
        self.validate()?;
        let url = client
            .endpoints()
            .resolve(crate::client::ApiFamily::PaasV4, &["chat", "completions"])?;
        let config = transport_config_from_client(client);
        let resp = send_json_request(
            reqwest::Method::POST,
            url,
            client.secret().expose(),
            &self.body,
            Arc::new(config),
        )
        .await?;
        parse_typed_response::<ChatCompletionResponse>(resp).await
    }

    /// Submit using a coding-plan endpoint via a [`ZaiClient`].
    pub async fn send_via_coding_plan(
        &self,
        client: &ZaiClient,
    ) -> crate::ZaiResult<ChatCompletionResponse>
    where
        N: serde::Serialize,
        M: serde::Serialize,
    {
        self.validate()?;
        let url = client.endpoints().resolve(
            crate::client::ApiFamily::CodingPaasV4,
            &["chat", "completions"],
        )?;
        let config = transport_config_from_client(client);
        let resp = send_json_request(
            reqwest::Method::POST,
            url,
            client.secret().expose(),
            &self.body,
            Arc::new(config),
        )
        .await?;
        parse_typed_response::<ChatCompletionResponse>(resp).await
    }
}

impl<N, M> ChatCompletion<N, M, StreamOn>
where
    N: ModelName + Chat,
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

    /// Disables streaming for this chat completion request.
    pub fn disable_stream(mut self) -> ChatCompletion<N, M, StreamOff> {
        self.body.stream = Some(false);
        self.body.tool_stream = None;
        ChatCompletion {
            body: self.body,
            _stream: PhantomData,
        }
    }

    /// Borrow the body for SSE processing.
    pub fn body(&self) -> &ChatBody<N, M> {
        &self.body
    }

    /// Resolve the URL and expose the serialized body for SSE streaming via a
    /// [`ZaiClient`]. Returns `(url, serialized_body, api_key)`.
    pub fn prepare_stream_via(
        &self,
        client: &ZaiClient,
    ) -> crate::ZaiResult<(String, Vec<u8>, String)>
    where
        N: serde::Serialize,
        M: serde::Serialize,
    {
        self.body
            .validate()
            .map_err(crate::client::error::ZaiError::from)?;
        let url = client
            .endpoints()
            .resolve(crate::client::ApiFamily::PaasV4, &["chat", "completions"])?;
        let body_bytes =
            serde_json::to_vec(&self.body).map_err(|e| crate::ZaiError::JsonError(Arc::new(e)))?;
        Ok((url, body_bytes, client.secret().expose().to_string()))
    }
}

/// Build a legacy `HttpClientConfig` from a `ZaiClient`'s transport policy.
/// This is a temporary bridge during P05–P06; once all endpoints route through
/// the new `Transport`, this adapter is removed.
fn transport_config_from_client(client: &ZaiClient) -> HttpClientConfig {
    let t = client.transport();
    HttpClientConfig {
        timeout: std::time::Duration::from_secs(t.request_timeout.as_secs()),
        max_retries: u32::from(t.max_attempts).saturating_sub(1),
        enable_compression: t.enable_compression,
        retry_delay: crate::client::http::RetryDelay::Exponential {
            base: std::time::Duration::from_millis(500),
            max: std::time::Duration::from_secs(5),
        },
        enable_logging: false,
        mask_sensitive_data: true,
    }
}
