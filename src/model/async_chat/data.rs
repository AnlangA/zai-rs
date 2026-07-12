use serde::Serialize;
use validator::Validate;

use super::super::{chat_base_request::*, tools::*, traits::*};
use crate::client::ZaiClient;

/// Builder for submitting a queued chat-completion task through a [`ZaiClient`].
///
/// Posts to the `async/chat/completions` task-submission endpoint.
pub struct AsyncChatCompletion<N, M>
where
    N: ModelName + AsyncChat,
    (N, M): Bounded,
    ChatBody<N, M>: Serialize,
{
    body: ChatBody<N, M>,
}

impl<N, M> AsyncChatCompletion<N, M>
where
    N: ModelName + AsyncChat,
    (N, M): Bounded,
    ChatBody<N, M>: Serialize,
{
    /// Create a new async chat request from a model and the first message batch.
    pub fn new(model: N, messages: M) -> Self {
        Self {
            body: ChatBody::new(model, messages),
        }
    }

    /// Borrow the request body mutably for options that are not exposed by this
    /// builder.
    pub fn body_mut(&mut self) -> &mut ChatBody<N, M> {
        &mut self.body
    }

    /// Append another message to the conversation.
    pub fn add_messages(mut self, messages: M) -> Self {
        self.body = self.body.add_messages(messages);
        self
    }
    /// Set the client-provided request identifier.
    pub fn with_request_id(mut self, request_id: impl Into<String>) -> Self {
        self.body = self.body.with_request_id(request_id);
        self
    }
    /// Enable or disable probabilistic sampling.
    pub fn with_do_sample(mut self, do_sample: bool) -> Self {
        self.body = self.body.with_do_sample(do_sample);
        self
    }
    /// Enable or disable incremental tool-call arguments.
    ///
    /// This option is available only for models implementing
    /// [`ToolStreamEnable`]. Passing `true` also sets `stream=true`; consequently
    /// [`validate`](Self::validate) rejects it for this queued endpoint.
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
    /// Set the nucleus-sampling probability.
    pub fn with_top_p(mut self, top_p: f64) -> Self {
        self.body = self.body.with_top_p(top_p);
        self
    }
    /// Set the maximum number of generated tokens.
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.body = self.body.with_max_tokens(max_tokens);
        self
    }
    /// Add one tool the model may call.
    pub fn add_tool(mut self, tool: Tools) -> Self {
        self.body = self.body.add_tool(tool);
        self
    }
    /// Add multiple tools the model may call.
    pub fn add_tools(mut self, tools: Vec<Tools>) -> Self {
        self.body = self.body.extend_tools(tools);
        self
    }
    /// Set the end-user identifier used for abuse monitoring.
    pub fn with_user_id(mut self, user_id: impl Into<String>) -> Self {
        self.body = self.body.with_user_id(user_id);
        self
    }
    /// Add the stop sequence for this request.
    pub fn with_stop(mut self, stop: String) -> Self {
        self.body = self.body.with_stop(stop);
        self
    }
    /// Configure thinking mode for a model that supports it.
    pub fn with_thinking(mut self, thinking: ThinkingType) -> Self
    where
        N: ThinkEnable,
    {
        self.body = self.body.with_thinking(thinking);
        self
    }
    /// Set reasoning depth for a model that supports `reasoning_effort`.
    pub fn with_reasoning_effort(mut self, effort: ReasoningEffort) -> Self
    where
        N: ReasoningEffortEnable,
    {
        self.body = self.body.with_reasoning_effort(effort);
        self
    }

    /// Validate field constraints and reject streaming on the queued endpoint.
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

    /// Submit via a [`ZaiClient`] and await the task-creation response.
    pub async fn send_via(
        &self,
        client: &ZaiClient,
    ) -> crate::ZaiResult<crate::model::chat_base_response::ChatCompletionResponse>
    where
        N: serde::Serialize,
        M: serde::Serialize,
    {
        self.validate()?;
        let route = crate::client::routes::CHAT_COMPLETE_ASYNC;
        let url = client.endpoints().resolve_route(route, &[])?;
        client
            .send_json::<_, crate::model::chat_base_response::ChatCompletionResponse>(
                route.method(),
                url,
                &self.body,
            )
            .await
    }
}
