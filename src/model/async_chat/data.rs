use super::super::chat_base_request::*;
use super::super::tools::*;
use super::super::traits::*;
use crate::client::http::HttpClient;
use serde::Serialize;
use std::marker::PhantomData;
use validator::Validate;

pub struct AsyncChatCompletion<N, M, S = StreamOff>
where
    N: ModelName + AsyncChat,
    (N, M): Bounded,
    ChatBody<N, M>: Serialize,
    S: StreamState,
{
    pub key: String,
    body: ChatBody<N, M>,
    _stream: PhantomData<S>,
}

impl<N, M> AsyncChatCompletion<N, M, StreamOff>
where
    N: ModelName + AsyncChat,
    (N, M): Bounded,
    ChatBody<N, M>: Serialize,
{
    pub fn new(model: N, messages: M, key: String) -> Self {
        let body = ChatBody::new(model, messages);
        Self {
            body,
            key,
            _stream: PhantomData,
        }
    }

    pub fn body_mut(&mut self) -> &mut ChatBody<N, M> {
        &mut self.body
    }

    // Fluent, builder-style forwarding methods to mutate inner ChatBody and return Self
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
    pub fn with_tool_stream(mut self, tool_stream: bool) -> Self
    where
        N: ToolStreamEnable,
    {
        self.body = self.body.with_tool_stream(tool_stream);
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

    // Type-state toggles
    pub fn enable_stream(mut self) -> AsyncChatCompletion<N, M, StreamOn> {
        self.body.stream = Some(true);
        AsyncChatCompletion {
            key: self.key,
            body: self.body,
            _stream: PhantomData,
        }
    }

    /// Validate request parameters for non-stream async chat (StreamOff)
    pub fn validate(&self) -> anyhow::Result<()> {
        self.body.validate().map_err(|e| anyhow::anyhow!(e))?;
        if matches!(self.body.stream, Some(true)) {
            return Err(anyhow::anyhow!(
                "stream=true detected; use enable_stream() and streaming APIs instead"
            ));
        }
        Ok(())
    }

    /// Send the request and parse typed response.
    /// Automatically runs `validate()` before sending.
    pub async fn send(
        &self,
    ) -> anyhow::Result<crate::model::chat_base_response::ChatCompletionResponse>
    where
        N: serde::Serialize,
        M: serde::Serialize,
    {
        self.validate()?;
        let resp: reqwest::Response = self.post().await?;
        let parsed = resp
            .json::<crate::model::chat_base_response::ChatCompletionResponse>()
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
    pub fn with_tool_stream(mut self, tool_stream: bool) -> Self
    where
        N: ToolStreamEnable,
    {
        self.body = self.body.with_tool_stream(tool_stream);
        self
    }

    pub fn disable_stream(mut self) -> AsyncChatCompletion<N, M, StreamOff> {
        self.body.stream = Some(false);
        // Reset tool_stream when disabling streaming since tool_stream depends on stream
        self.body.tool_stream = None;
        AsyncChatCompletion {
            key: self.key,
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
    type ApiUrl = &'static str;
    type ApiKey = String;

    fn api_url(&self) -> &Self::ApiUrl {
        &"https://open.bigmodel.cn/api/paas/v4/async/chat/completions"
    }
    fn api_key(&self) -> &Self::ApiKey {
        &self.key
    }
    fn body(&self) -> &Self::Body {
        &self.body
    }
}

impl<N, M> crate::model::traits::SseStreamable for AsyncChatCompletion<N, M, StreamOn>
where
    N: ModelName + Serialize + AsyncChat,
    M: Serialize,
    (N, M): Bounded,
{
}

// Enable typed streaming extension methods for AsyncChatCompletion<..., StreamOn>
impl<N, M> crate::model::stream_ext::StreamChatLikeExt for AsyncChatCompletion<N, M, StreamOn>
where
    N: ModelName + Serialize + AsyncChat,
    M: Serialize,
    (N, M): Bounded,
{
}
