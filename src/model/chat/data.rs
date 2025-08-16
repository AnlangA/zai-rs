use super::super::base::*;
use super::super::traits::*;
use crate::client::http::HttpClient;
use serde::Serialize;
pub struct ChatCompletion<N, M>
where
    N: ModelName,
    (N, M): Bounded,
    ChatBody<N, M>: Serialize,
{
    pub key: String,
    body: ChatBody<N, M>,
}

impl<N, M> ChatCompletion<N, M>
where
    N: ModelName,
    (N, M): Bounded,
    ChatBody<N, M>: Serialize,
{
    pub fn new(model: N, messages: M, key: String) -> Self {
        let body = ChatBody::new(model, messages);
        Self { body, key }
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
    pub fn with_tools(mut self, tools: impl Into<Vec<Tools>>) -> Self {
        self.body = self.body.with_tools(tools);
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
}

impl<N, M> HttpClient for ChatCompletion<N, M>
where
    N: ModelName + Serialize,
    M: Serialize,
    (N, M): Bounded,
{
    type Body = ChatBody<N, M>;
    type ApiUrl = &'static str;
    type ApiKey = String;

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
