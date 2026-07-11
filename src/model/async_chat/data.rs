use std::sync::Arc;

use serde::Serialize;
use validator::Validate;

use super::super::{chat_base_request::*, tools::*, traits::*};
use crate::client::http::{HttpClientConfig, parse_typed_response, send_json_request};
use crate::client::v2::ZaiClient;

/// Asynchronous (queued) chat-completion request builder (P05: routes through
/// [`ZaiClient`]).
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

    pub fn body_mut(&mut self) -> &mut ChatBody<N, M> {
        &mut self.body
    }

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
    pub fn with_tool_stream(mut self, tool_stream: bool) -> Self
    where
        N: ToolStreamEnable,
    {
        self.body = self.body.with_tool_stream(tool_stream);
        self
    }
    pub fn with_temperature(mut self, temperature: f64) -> Self {
        self.body = self.body.with_temperature(temperature);
        self
    }
    pub fn with_top_p(mut self, top_p: f64) -> Self {
        self.body = self.body.with_top_p(top_p);
        self
    }
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.body = self.body.with_max_tokens(max_tokens);
        self
    }
    pub fn add_tool(mut self, tool: Tools) -> Self {
        self.body = self.body.add_tool(tool);
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
    pub fn with_thinking(mut self, thinking: ThinkingType) -> Self
    where
        N: ThinkEnable,
    {
        self.body = self.body.with_thinking(thinking);
        self
    }
    pub fn with_reasoning_effort(mut self, effort: ReasoningEffort) -> Self
    where
        N: ReasoningEffortEnable,
    {
        self.body = self.body.with_reasoning_effort(effort);
        self
    }

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
        let url = client.endpoints().resolve(
            crate::client::v2::ApiFamily::PaasV4,
            &["async", "chat", "completions"],
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
        parse_typed_response::<crate::model::chat_base_response::ChatCompletionResponse>(resp).await
    }
}

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
