use std::marker::PhantomData;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures_util::{Stream, StreamExt};
use serde::Serialize;
use validator::Validate;

use super::super::{
    chat_base_request::*, chat_base_response::ChatCompletionResponse,
    chat_stream_response::ChatStreamResponse, tools::*, traits::*,
};
use crate::client::ZaiClient;

/// Authenticated typed response stream returned by [`ChatCompletion::stream_via`].
///
/// [`next`](Self::next) is available without importing an extension trait. The
/// type also implements [`Stream`] for combinators from `futures-util`.
pub struct ChatStream {
    inner: Pin<Box<dyn Stream<Item = crate::ZaiResult<ChatStreamResponse>> + Send + 'static>>,
}

impl ChatStream {
    /// Await the next decoded chat chunk, or `None` after `[DONE]`.
    ///
    /// An unexpected EOF is yielded as an error before the stream terminates.
    pub async fn next(&mut self) -> Option<crate::ZaiResult<ChatStreamResponse>> {
        self.inner.next().await
    }
}

impl Stream for ChatStream {
    type Item = crate::ZaiResult<ChatStreamResponse>;

    fn poll_next(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.inner.as_mut().poll_next(context)
    }
}

/// Typed chat-completion request builder sent through a [`ZaiClient`].
///
/// Generic over the model `N`, the message type `M`, and a stream type-state
/// `S` (`StreamOff` by default, `StreamOn` after
/// [`enable_stream`](Self::enable_stream)).
/// Credentials, endpoint selection and transport policy are owned by the
/// [`ZaiClient`] supplied to [`send_via`](Self::send_via).
///
/// Audio chat deliberately has no tool surface:
///
/// ```compile_fail
/// use zai_rs::model::{
///     chat::ChatCompletion,
///     chat_message_types::VoiceMessage,
///     chat_models::GLM4_voice,
///     tools::Function,
/// };
///
/// ChatCompletion::new(GLM4_voice {}, VoiceMessage::new_user()).add_tool(
///     Function::new("lookup", "Lookup", serde_json::json!({"type": "object"})),
/// );
/// ```
///
/// ```compile_fail
/// use zai_rs::model::{
///     chat::ChatCompletion,
///     chat_message_types::VoiceMessage,
///     chat_models::GLM4_voice,
///     tools::ToolChoice,
/// };
///
/// ChatCompletion::new(GLM4_voice {}, VoiceMessage::new_user())
///     .with_tool_choice(ToolChoice::auto());
/// ```
///
/// ```compile_fail
/// use zai_rs::model::{
///     chat::ChatCompletion,
///     chat_message_types::VoiceMessage,
///     chat_models::GLM4_voice,
///     tools::ResponseFormat,
/// };
///
/// ChatCompletion::new(GLM4_voice {}, VoiceMessage::new_user())
///     .with_response_format(ResponseFormat::JsonObject);
/// ```
///
/// Vision chat accepts a bare [`Function`](crate::model::tools::Function), not
/// the text schema's retrieval, web-search, or MCP tool variants:
///
/// ```compile_fail
/// use zai_rs::model::{
///     chat::ChatCompletion,
///     chat_message_types::VisionMessage,
///     chat_models::GLM4_6v,
///     tools::{Retrieval, Tools},
/// };
///
/// ChatCompletion::new(GLM4_6v {}, VisionMessage::new_user()).add_tool(
///     Tools::Retrieval { retrieval: Retrieval::new("kb") },
/// );
/// ```
///
/// ```compile_fail
/// use zai_rs::model::{
///     chat::ChatCompletion,
///     chat_message_types::VisionMessage,
///     chat_models::GLM4_6v,
///     tools::ResponseFormat,
/// };
///
/// ChatCompletion::new(GLM4_6v {}, VisionMessage::new_user())
///     .with_response_format(ResponseFormat::JsonObject);
/// ```
pub struct ChatCompletion<N, M, S = StreamOff>
where
    N: ChatRequestModel + Chat,
    M: Serialize,
    (N, M): Bounded,
    ChatBody<N, M>: Serialize,
    S: StreamState,
{
    body: ChatBody<N, M>,
    _stream: PhantomData<S>,
}

impl<N, M, S> ChatCompletion<N, M, S>
where
    N: ChatRequestModel + Chat,
    M: Serialize,
    (N, M): Bounded,
    ChatBody<N, M>: Serialize,
    S: StreamState,
{
    /// Borrow the frozen request body.
    pub const fn body(&self) -> &ChatBody<N, M> {
        &self.body
    }
}

impl<N, M> ChatCompletion<N, M, StreamOff>
where
    N: ChatRequestModel + Chat,
    M: Serialize,
    (N, M): Bounded,
    ChatBody<N, M>: Serialize,
{
    /// Create a new chat request from a model and the first message batch.
    pub fn new(model: N, messages: M) -> ChatCompletion<N, M, StreamOff> {
        let body = ChatBody::new(model, messages);
        ChatCompletion {
            body,
            _stream: PhantomData,
        }
    }

    /// Append one message to the conversation.
    pub fn add_message(mut self, message: M) -> Self {
        self.body = self.body.add_message(message);
        self
    }
    /// Append multiple messages to the conversation.
    pub fn extend_messages(mut self, messages: impl IntoIterator<Item = M>) -> Self {
        self.body = self.body.extend_messages(messages);
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
    pub fn add_tool(mut self, tool: N::Tool) -> Self
    where
        N: ChatToolSupport,
    {
        self.body = self.body.add_tool(tool);
        self
    }
    /// Add multiple tools to the request at once.
    pub fn add_tools(mut self, tools: impl IntoIterator<Item = N::Tool>) -> Self
    where
        N: ChatToolSupport,
    {
        self.body = self.body.add_tools(tools);
        self
    }
    /// Set automatic tool selection.
    pub fn with_tool_choice(mut self, tool_choice: ToolChoice) -> Self
    where
        N: ChatToolSupport,
    {
        self.body = self.body.with_tool_choice(tool_choice);
        self
    }
    /// Remove all tools and their selection policy.
    pub fn clear_tools(mut self) -> Self
    where
        N: ChatToolSupport,
    {
        self.body = self.body.clear_tools();
        self
    }
    /// Set the text response format.
    pub fn with_response_format(mut self, format: ResponseFormat) -> Self
    where
        N: ResponseFormatEnable,
    {
        self.body = self.body.with_response_format(format);
        self
    }
    /// Enable or disable audio-output watermarking.
    pub fn with_watermark_enabled(mut self, enabled: bool) -> Self
    where
        N: WatermarkEnable,
    {
        self.body = self.body.with_watermark_enabled(enabled);
        self
    }
    /// Set the end-user id (used for abuse monitoring).
    pub fn with_user_id(mut self, user_id: impl Into<String>) -> Self {
        self.body = self.body.with_user_id(user_id);
        self
    }
    /// Add a stop sequence that halts generation when encountered.
    pub fn with_stop(mut self, stop: impl Into<String>) -> Self {
        self.body = self.body.with_stop(stop);
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

    /// Enables streaming for this chat completion request.
    pub fn enable_stream(mut self) -> ChatCompletion<N, M, StreamOn> {
        self.body.set_stream(Some(true));
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
        if matches!(self.body.stream(), Some(true)) {
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
    pub async fn send_via(&self, client: &ZaiClient) -> crate::ZaiResult<ChatCompletionResponse>
    where
        N: serde::Serialize,
        M: serde::Serialize,
    {
        self.validate()?;
        let route = crate::client::routes::CHAT_COMPLETE;
        client
            .operation(route)
            .send_json::<_, ChatCompletionResponse>(&self.body)
            .await
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
        let route = crate::client::routes::CHAT_COMPLETE_CODING;
        client
            .operation(route)
            .send_json::<_, ChatCompletionResponse>(&self.body)
            .await
    }
}

impl<N, M> ChatCompletion<N, M, StreamOn>
where
    N: ChatRequestModel + Chat,
    M: Serialize,
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
        self.body.set_stream(Some(false));
        self.body.clear_tool_stream();
        ChatCompletion {
            body: self.body,
            _stream: PhantomData,
        }
    }

    /// Submit the request and yield parsed chat chunks from its SSE response.
    ///
    /// Authentication, response content-type validation, timeouts, and body
    /// limits remain inside [`ZaiClient`]; the API secret is never exposed to
    /// the caller. Streaming POST requests are not retried or redirected.
    pub async fn stream_via(&self, client: &ZaiClient) -> crate::ZaiResult<ChatStream>
    where
        N: serde::Serialize,
        M: serde::Serialize,
    {
        self.body
            .validate()
            .map_err(crate::client::error::ZaiError::from)?;
        let raw = client
            .operation(crate::client::routes::CHAT_COMPLETE)
            .send_sse_json(&self.body)
            .await?;
        let stream = crate::model::sse_parser::decode_required_done_stream(raw, |payload| {
            serde_json::from_slice::<ChatStreamResponse>(payload)
                .map_err(crate::ZaiError::from)
                .and_then(validate_stream_finish_reason)
        });
        Ok(ChatStream { inner: stream })
    }
}

fn validate_stream_finish_reason(
    chunk: ChatStreamResponse,
) -> crate::ZaiResult<ChatStreamResponse> {
    let failure = chunk
        .choices
        .iter()
        .filter_map(|choice| choice.finish_reason.as_deref())
        .find_map(|reason| match reason {
            "sensitive" => Some((1301, "stream stopped by content policy")),
            "network_error" => Some((1234, "stream stopped by an upstream inference error")),
            "model_context_window_exceeded" => {
                Some((1261, "stream exceeded the model context window"))
            },
            _ => None,
        });

    match failure {
        Some((code, message)) => Err(crate::ZaiError::from_api_response(
            200,
            code,
            message.to_string(),
        )),
        None => Ok(chunk),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{chat_message_types::TextMessage, chat_models::GLM5_2};

    #[test]
    fn stream_field_is_owned_by_the_type_state() {
        let request = ChatCompletion::new(GLM5_2 {}, TextMessage::user("hello"));
        let json = serde_json::to_value(request.body()).unwrap();
        assert!(json.get("stream").is_none());
        assert!(json.get("tool_stream").is_none());

        let request = request.enable_stream().with_tool_stream(true);
        let json = serde_json::to_value(request.body()).unwrap();
        assert_eq!(json["stream"], true);
        assert_eq!(json["tool_stream"], true);
    }
}
