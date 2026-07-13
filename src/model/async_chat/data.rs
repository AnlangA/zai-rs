use serde::Serialize;
use validator::Validate;

use super::super::{chat_base_request::*, tools::*, traits::*};
use crate::{client::ZaiClient, model::async_chat_get::AsyncResponse};

/// Builder for submitting a queued chat-completion task through a [`ZaiClient`].
///
/// Posts to the `async/chat/completions` task-submission endpoint.
pub struct AsyncChatCompletion<N, M>
where
    N: ChatRequestModel + AsyncChat,
    M: Serialize,
    (N, M): Bounded,
    ChatBody<N, M>: Serialize,
{
    body: ChatBody<N, M>,
}

impl<N, M> AsyncChatCompletion<N, M>
where
    N: ChatRequestModel + AsyncChat,
    M: Serialize,
    (N, M): Bounded,
    ChatBody<N, M>: Serialize,
{
    /// Create a new async chat request from a model and the first message batch.
    pub fn new(model: N, messages: M) -> Self {
        Self {
            body: ChatBody::new(model, messages),
        }
    }

    /// Borrow the frozen request body.
    pub const fn body(&self) -> &ChatBody<N, M> {
        &self.body
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
    pub fn add_tool(mut self, tool: N::Tool) -> Self
    where
        N: ChatToolSupport,
    {
        self.body = self.body.add_tool(tool);
        self
    }
    /// Add multiple tools the model may call.
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
    /// Set the response format for a text model.
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
    /// Set the end-user identifier used for abuse monitoring.
    pub fn with_user_id(mut self, user_id: impl Into<String>) -> Self {
        self.body = self.body.with_user_id(user_id);
        self
    }
    /// Add the stop sequence for this request.
    pub fn with_stop(mut self, stop: impl Into<String>) -> Self {
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

    /// Validate field constraints for the queued endpoint.
    pub fn validate(&self) -> crate::ZaiResult<()> {
        self.body
            .validate()
            .map_err(crate::client::error::ZaiError::from)?;
        Ok(())
    }

    /// Submit via a [`ZaiClient`] and await the task-creation response.
    pub async fn send_via(&self, client: &ZaiClient) -> crate::ZaiResult<AsyncResponse>
    where
        N: serde::Serialize,
        M: serde::Serialize,
    {
        self.validate()?;
        let route = crate::client::routes::CHAT_COMPLETE_ASYNC;
        let response = client
            .operation(route)
            .send_json::<_, AsyncResponse>(&self.body)
            .await?;
        response.validate()?;
        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        chat_message_types::{TextMessage, VoiceMessage},
        chat_models::{GLM4_voice, GLM5_2},
    };

    #[test]
    fn async_text_schema_never_serializes_stream_fields() {
        let request = AsyncChatCompletion::new(GLM5_2 {}, TextMessage::user("hello"))
            .add_tool(Tools::Function {
                function: Function::new("lookup", "Lookup", serde_json::json!({"type": "object"})),
            })
            .with_tool_choice(ToolChoice::auto())
            .with_response_format(ResponseFormat::JsonObject);
        let json = serde_json::to_value(request.body()).unwrap();

        assert_eq!(json["tool_choice"], "auto");
        assert_eq!(json["response_format"]["type"], "json_object");
        assert!(json.get("stream").is_none());
        assert!(json.get("tool_stream").is_none());
        assert!(json.get("watermark_enabled").is_none());
    }

    #[test]
    fn async_audio_schema_serializes_only_its_specialized_field() {
        let request = AsyncChatCompletion::new(GLM4_voice {}, VoiceMessage::new_user())
            .with_watermark_enabled(true);
        let json = serde_json::to_value(request.body()).unwrap();

        assert_eq!(json["watermark_enabled"], true);
        for field in [
            "stream",
            "tool_stream",
            "tools",
            "tool_choice",
            "response_format",
        ] {
            assert!(
                json.get(field).is_none(),
                "unexpected async audio field: {field}"
            );
        }
    }
}
