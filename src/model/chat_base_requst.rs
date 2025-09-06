//! Base types and structures for chat API models.
//!
//! This module provides the core data structures used for chat API requests,
//! including the main request body structure.

use super::tools::*;
use super::traits::*;
use serde::Serialize;
use validator::*;

/// Main request body structure for chat API calls.
///
/// This structure represents a complete chat request with all possible configuration options.
/// It uses generic types to support different model names and message types while maintaining
/// type safety through trait bounds.
///
/// # Type Parameters
///
/// * `N` - The model name type, must implement [`ModelName`]
/// * `M` - The message type, must form a [`Bounded`] pair with `N`
///
/// # Examples
///
/// ```rust,ignore
/// use crate::model::base::{ChatBody, TextMessage};
///
/// // Create a basic chat request
/// let chat_body = ChatBody {
///     model: "gpt-4".to_string(),
///     messages: vec![
///         TextMessage::user("Hello, how are you?"),
///         TextMessage::assistant("I'm doing well, thank you!")
///     ],
///     temperature: Some(0.7),
///     max_tokens: Some(1000),
///     ..Default::default()
/// };
/// ```
#[derive(Debug, Clone, Validate, Serialize)]
pub struct ChatBody<N, M>
where
    N: ModelName,
    (N, M): Bounded,
{
    /// The model to use for the chat completion.
    pub model: N,

    /// A list of messages comprising the conversation so far.
    pub messages: Vec<M>,

    /// A unique identifier for the request. Optional field that will be omitted from
    /// serialization if not provided.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,

    /// Optional thinking prompt or reasoning text that can guide the model's response.
    /// Only available for models that support thinking capabilities.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking: Option<ThinkingType>,

    /// Whether to use sampling during generation. When `true`, the model will use
    /// probabilistic sampling; when `false`, it will use deterministic generation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub do_sample: Option<bool>,

    /// Whether to stream back partial message deltas as they are generated.
    /// When `true`, responses will be sent as server-sent events.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,

    /// Controls randomness in the output. Higher values (closer to 1.0) make the output
    /// more random, while lower values (closer to 0.0) make it more deterministic.
    /// Must be between 0.0 and 1.0.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(range(min = 0.0, max = 1.0))]
    pub temperature: Option<f32>,

    /// Controls diversity via nucleus sampling. Only tokens with cumulative probability
    /// up to `top_p` are considered. Must be between 0.0 and 1.0.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(range(min = 0.0, max = 1.0))]
    pub top_p: Option<f32>,

    /// The maximum number of tokens to generate in the completion.
    /// Must be between 1 and 98304.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(range(min = 1, max = 98304))]
    pub max_tokens: Option<u32>,

    /// A list of tools the model may call. Currently supports function calling,
    /// web search, and retrieval tools.
    /// Note: server expects an array; we model this as a vector of tool items.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tools>>,

    // tool_choice: enum<string>, but we don't need it for now
    /// A unique identifier representing your end-user, which can help monitor and
    /// detect abuse. Must be between 6 and 128 characters long.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(length(min = 6, max = 128))]
    pub user_id: Option<String>,

    /// Up to 1 sequence where the API will stop generating further tokens.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(length(min = 1, max = 1))]
    pub stop: Option<Vec<String>>,

    /// An object specifying the format that the model must output.
    /// Can be either text or JSON object format.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<ResponseFormat>,
}

impl<N, M> ChatBody<N, M>
where
    N: ModelName,
    (N, M): Bounded,
{
    pub fn new(model: N, messages: M) -> Self {
        Self {
            model,
            messages: vec![messages],
            request_id: None,
            thinking: None,
            do_sample: None,
            stream: None,
            temperature: None,
            top_p: None,
            max_tokens: None,
            tools: None,
            user_id: None,
            stop: None,
            response_format: None,
        }
    }

    pub fn add_messages(mut self, messages: M) -> Self {
        self.messages.push(messages);
        self
    }
    pub fn with_request_id(mut self, request_id: impl Into<String>) -> Self {
        self.request_id = Some(request_id.into());
        self
    }
    pub fn with_do_sample(mut self, do_sample: bool) -> Self {
        self.do_sample = Some(do_sample);
        self
    }
    pub fn with_stream(mut self, stream: bool) -> Self {
        self.stream = Some(stream);
        self
    }
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }
    pub fn with_top_p(mut self, top_p: f32) -> Self {
        self.top_p = Some(top_p);
        self
    }
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }
    pub fn with_tools(mut self, tools: impl Into<Vec<Tools>>) -> Self {
        self.tools = Some(tools.into());
        self
    }
    pub fn add_tools(mut self, tools: Tools) -> Self {
        self.tools.get_or_insert(Vec::new()).push(tools);
        self
    }
    pub fn extend_tools(mut self, tools: Vec<Tools>) -> Self {
        self.tools.get_or_insert(Vec::new()).extend(tools);
        self
    }
    pub fn with_user_id(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = Some(user_id.into());
        self
    }
    pub fn with_stop(mut self, stop: String) -> Self {
        self.stop.get_or_insert_with(Vec::new).push(stop);
        self
    }
}

impl<N, M> ChatBody<N, M>
where
    N: ModelName + ThinkEnable,
    (N, M): Bounded,
{
    /// Adds thinking text to the chat body for models that support thinking capabilities.
    ///
    /// This method is only available for models that implement the [`ThinkEnable`] trait,
    /// ensuring type safety for thinking-enabled models.
    ///
    /// # Arguments
    ///
    /// * `thinking` - The thinking prompt or reasoning text to add
    ///
    /// # Returns
    ///
    /// Returns `self` with the thinking field set, allowing for method chaining.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let chat_body = ChatBody::new(model, messages)
    ///     .with_thinking("Let me think step by step about this problem...");
    /// ```
    pub fn with_thinking(mut self, thinking: ThinkingType) -> Self {
        self.thinking = Some(thinking);
        self
    }
}

// 为方便使用，实现从单个Tools到Vec<Tools>的转换
impl From<Tools> for Vec<Tools> {
    fn from(tool: Tools) -> Self {
        vec![tool]
    }
}