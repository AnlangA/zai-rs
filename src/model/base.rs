//! Base types and structures for chat API models.
//!
//! This module provides the core data structures used for chat API requests and responses,
//! including message types, tool calls, and configuration options.

use super::model_validate::validate_json_schema;
use super::traits::*;
use serde::ser::Error;
use serde::{Deserialize, Serialize, Serializer};
use std::fmt::Debug;
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
    pub thinking: Option<String>,

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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Tools>,

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
    pub fn with_tools(mut self, tools: Tools) -> Self {
        self.tools = Some(tools);
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
    pub fn with_thinking(mut self, thinking: impl Into<String>) -> Self {
        self.thinking = Some(thinking.into());
        self
    }
}

/// A collection of text messages with validation constraints.
///
/// This structure wraps a vector of [`TextMessage`] instances and ensures that
/// the collection contains between 1 and 1000 messages, as required by most
/// chat API endpoints.
///
/// # Validation
///
/// - Must contain at least 1 message
/// - Must not contain more than 1000 messages
#[derive(Clone, Serialize, Validate)]
pub struct TextMessages {
    /// The collection of text messages. Must contain between 1 and 1000 messages.
    #[validate(length(min = 1, max = 1000))]
    pub messages: Vec<TextMessage>,
}

impl TextMessages {
    /// Creates a new `TextMessages` collection with a single message.
    ///
    /// # Arguments
    ///
    /// * `messages` - The initial message to include in the collection
    ///
    /// # Returns
    ///
    /// A new `TextMessages` instance containing the provided message.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let messages = TextMessages::new(TextMessage::user("Hello!"));
    /// ```
    pub fn new(messages: TextMessage) -> Self {
        Self {
            messages: vec![messages],
        }
    }

    /// Adds a message to the collection.
    ///
    /// This method provides a fluent interface for building message collections
    /// while maintaining encapsulation of the internal Vec structure.
    ///
    /// # Arguments
    ///
    /// * `msg` - The message to add to the collection
    ///
    /// # Returns
    ///
    /// Returns `self` with the new message added, allowing for method chaining.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let messages = TextMessages::new(TextMessage::user("Hello!"))
    ///     .add_message(TextMessage::assistant("Hi there!"))
    ///     .add_message(TextMessage::user("How are you?"));
    /// ```
    pub fn add_message(mut self, msg: TextMessage) -> Self {
        self.messages.push(msg);
        self
    }
}

/// Represents different types of messages in a chat conversation.
///
/// This enum defines the four main types of messages that can appear in a chat:
/// user messages, assistant responses, system instructions, and tool responses.
/// Each variant is serialized with a "role" field to distinguish message types.
///
/// # Serialization
///
/// Messages are serialized as JSON objects with a "role" field that indicates
/// the message type ("user", "assistant", "system", or "tool").
///
/// # Examples
///
/// ```rust,ignore
/// // Create different types of messages
/// let user_msg = TextMessage::user("What's the weather like?");
/// let system_msg = TextMessage::system("You are a helpful assistant.");
/// let assistant_msg = TextMessage::assistant("I can help you with that!");
/// ```
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "role")]
#[serde(rename_all = "lowercase")]
pub enum TextMessage {
    /// A message from the user/human in the conversation.
    User {
        /// The content of the user's message.
        content: String,
    },
    /// A response from the AI assistant.
    Assistant {
        /// The text content of the assistant's response. Optional when tool calls are present.
        #[serde(skip_serializing_if = "Option::is_none")]
        content: Option<String>,
        /// Tool calls made by the assistant. Empty vector is omitted from serialization.
        #[serde(skip_serializing_if = "Vec::is_empty")]
        tool_calls: Vec<ToolCall>,
    },
    /// A system message that provides instructions or context to the assistant.
    System {
        /// The content of the system message.
        content: String,
    },
    /// A message containing the result of a tool call.
    Tool {
        /// The content returned by the tool.
        content: String,
        /// The ID of the tool call this message is responding to. Optional field.
        #[serde(skip_serializing_if = "Option::is_none")]
        tool_call_id: Option<String>,
    },
}

impl TextMessage {
    /// Creates a new user message.
    ///
    /// # Arguments
    ///
    /// * `content` - The content of the user's message
    ///
    /// # Returns
    ///
    /// A new `TextMessage::User` variant.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let msg = TextMessage::user("Hello, how can you help me today?");
    /// ```
    pub fn user(content: impl Into<String>) -> Self {
        TextMessage::User {
            content: content.into(),
        }
    }

    /// Creates a new assistant message with text content only.
    ///
    /// # Arguments
    ///
    /// * `content` - The content of the assistant's response
    ///
    /// # Returns
    ///
    /// A new `TextMessage::Assistant` variant with content and no tool calls.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let msg = TextMessage::assistant("I'm happy to help you with that!");
    /// ```
    pub fn assistant(content: impl Into<String>) -> Self {
        TextMessage::Assistant {
            content: Some(content.into()),
            tool_calls: Vec::new(),
        }
    }

    /// Creates a new assistant message with optional content and tool calls.
    ///
    /// This method is useful when the assistant needs to call tools or when
    /// the response consists entirely of tool calls without additional text.
    ///
    /// # Arguments
    ///
    /// * `content` - Optional text content for the assistant's response
    /// * `tool_calls` - Vector of tool calls made by the assistant
    ///
    /// # Returns
    ///
    /// A new `TextMessage::Assistant` variant with the specified content and tool calls.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let tool_call = ToolCall::new_function("call_123",
    ///     FunctionParams::new("get_weather", r#"{"location": "Tokyo"}"#));
    /// let msg = TextMessage::assistant_with_tools(
    ///     Some("Let me check the weather for you.".to_string()),
    ///     vec![tool_call]
    /// );
    /// ```
    pub fn assistant_with_tools(content: Option<String>, tool_calls: Vec<ToolCall>) -> Self {
        TextMessage::Assistant {
            content,
            tool_calls,
        }
    }

    /// Creates a new system message.
    ///
    /// System messages provide instructions or context to the assistant and
    /// typically appear at the beginning of a conversation.
    ///
    /// # Arguments
    ///
    /// * `content` - The content of the system message
    ///
    /// # Returns
    ///
    /// A new `TextMessage::System` variant.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let msg = TextMessage::system("You are a helpful assistant specialized in programming.");
    /// ```
    pub fn system(content: impl Into<String>) -> Self {
        TextMessage::System {
            content: content.into(),
        }
    }

    /// Creates a new tool message without a tool call ID.
    ///
    /// # Arguments
    ///
    /// * `content` - The content returned by the tool
    ///
    /// # Returns
    ///
    /// A new `TextMessage::Tool` variant without a tool call ID.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let msg = TextMessage::tool("The current temperature is 22°C");
    /// ```
    pub fn tool(content: impl Into<String>) -> Self {
        TextMessage::Tool {
            content: content.into(),
            tool_call_id: None,
        }
    }

    /// Creates a new tool message with a specific tool call ID.
    ///
    /// This method should be used when responding to a specific tool call
    /// made by the assistant, linking the tool response to the original call.
    ///
    /// # Arguments
    ///
    /// * `content` - The content returned by the tool
    /// * `tool_call_id` - The ID of the tool call this message is responding to
    ///
    /// # Returns
    ///
    /// A new `TextMessage::Tool` variant with the specified tool call ID.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let msg = TextMessage::tool_with_id(
    ///     "The current temperature is 22°C",
    ///     "call_123"
    /// );
    /// ```
    pub fn tool_with_id(content: impl Into<String>, tool_call_id: impl Into<String>) -> Self {
        TextMessage::Tool {
            content: content.into(),
            tool_call_id: Some(tool_call_id.into()),
        }
    }
}

/// Represents a tool call made by the assistant.
///
/// Tool calls allow the assistant to invoke external functions, perform web searches,
/// or access retrieval systems. Each tool call has a unique ID and a specific type
/// that determines what kind of operation is being performed.
///
/// # Fields
///
/// * `id` - A unique identifier for this tool call
/// * `type_` - The type of tool being called (function, web search, or retrieval)
/// * `function` - Function parameters, required when `type_` is `Function`
///
/// # Serialization
///
/// The struct implements custom serialization logic to ensure that the `function`
/// field is only included when appropriate for the tool call type.
#[derive(Debug, Clone)]
pub struct ToolCall {
    id: String,
    type_: ToolCallType,
    function: Option<FunctionParams>,
}

impl Serialize for ToolCall {
    /// Custom serialization implementation for `ToolCall`.
    ///
    /// This implementation ensures that:
    /// - The `function` field is required when `type_` is `Function`
    /// - The `function` field is only included when appropriate for the tool type
    /// - Validation errors are returned for invalid combinations
    ///
    /// # Errors
    ///
    /// Returns a serialization error if `type_` is `Function` but `function` is `None`.
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeStruct;

        // Validation: function field is required when type_ is Function
        if matches!(self.type_, ToolCallType::Function) && self.function.is_none() {
            return Err(Error::custom(
                "function field is required when type is 'function'",
            ));
        }

        let mut state = serializer.serialize_struct("ToolCall", 3)?;
        state.serialize_field("id", &self.id)?;
        state.serialize_field("type", &self.type_)?;

        // Include function field based on tool call type
        match self.type_ {
            ToolCallType::Function => {
                // Function type requires function field (validated above)
                state.serialize_field("function", &self.function)?;
            }
            _ => {
                // Other types only include function field if present
                if self.function.is_some() {
                    state.serialize_field("function", &self.function)?;
                }
            }
        }

        state.end()
    }
}

/// Specifies the type of tool being called.
///
/// This enum defines the different types of tools that can be invoked by the assistant.
/// Each type corresponds to a different capability or service.
///
/// # Variants
///
/// * `Function` - Call a user-defined function with specific parameters
/// * `WebSearch` - Perform a web search operation
/// * `Retrieval` - Access a retrieval/knowledge system
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolCallType {
    /// A function call with custom parameters.
    Function,
    /// A web search operation.
    WebSearch,
    /// A retrieval system access.
    Retrieval,
}

/// Parameters for a function call.
///
/// This structure contains the information needed to invoke a specific function,
/// including the function name and its arguments serialized as a JSON string.
///
/// # Fields
///
/// * `name` - The name of the function to call
/// * `arguments` - JSON string containing the function arguments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionParams {
    name: String,
    arguments: String,
}

impl ToolCall {
    pub fn new_function(id: impl Into<String>, function: FunctionParams) -> Self {
        Self {
            id: id.into(),
            type_: ToolCallType::Function,
            function: Some(function),
        }
    }

    pub fn new_web_search(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            type_: ToolCallType::WebSearch,
            function: None,
        }
    }

    pub fn new_retrieval(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            type_: ToolCallType::Retrieval,
            function: None,
        }
    }
}

impl FunctionParams {
    pub fn new(name: impl Into<String>, arguments: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            arguments: arguments.into(),
        }
    }
}

/// Configuration for thinking capabilities in models.
///
/// This enum controls whether a model should use thinking/reasoning capabilities
/// when processing requests.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
#[serde(tag = "type")]
pub enum ThinkingType {
    /// Enable thinking capabilities.
    Enable,
    /// Disable thinking capabilities.
    Disable,
}

/// Defines the available tools that can be used by the assistant.
///
/// This enum specifies different categories of tools that the assistant can invoke
/// during a conversation. Each variant contains a vector of specific tool configurations.
///
/// # Variants
///
/// * `FunctionCall` - Custom functions that can be called with parameters
/// * `Retrieval` - Access to retrieval/knowledge systems
/// * `WebSearch` - Web search capabilities
/// * `MCP` - Model Context Protocol tools
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum Tools {
    /// Function calling tools with custom parameters.
    FunctionCall { function_call: Vec<FunctionCall> },
    /// Retrieval system access tools.
    Retrieval { retrieval: Vec<Retrieval> },
    /// Web search tools.
    WebSearch { web_search: Vec<WebSearch> },
    /// Model Context Protocol tools.
    MCP { mcp: Vec<MCP> },
}

/// Definition of a callable function tool.
///
/// This structure defines a function that can be called by the assistant,
/// including its name, description, and parameter schema.
///
/// # Validation
///
/// * `name` - Must be between 1 and 64 characters
/// * `parameters` - Must be a valid JSON schema
#[derive(Debug, Clone, Serialize, Validate)]
pub struct FunctionCall {
    /// The name of the function. Must be between 1 and 64 characters.
    #[validate(length(min = 1, max = 64))]
    pub name: String,

    /// A description of what the function does.
    pub description: String,

    /// JSON schema describing the function's parameters.
    #[validate(custom(function = "validate_json_schema"))]
    pub parameters: String,
}

impl FunctionCall {
    /// Creates a new function call definition.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the function
    /// * `description` - A description of what the function does
    /// * `parameters` - JSON schema string describing the function parameters
    ///
    /// # Returns
    ///
    /// A new `FunctionCall` instance.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let func = FunctionCall::new(
    ///     "get_weather",
    ///     "Get current weather for a location",
    ///     r#"{"type": "object", "properties": {"location": {"type": "string"}}}"#
    /// );
    /// ```
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        parameters: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            parameters: parameters.into(),
        }
    }
}

/// Configuration for retrieval tool capabilities.
///
/// This structure represents a retrieval tool that can access knowledge bases
/// or document collections. Currently a placeholder for future expansion.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct Retrieval {}

/// Configuration for web search tool capabilities.
///
/// This structure represents a web search tool that can perform internet searches.
/// Currently a placeholder for future expansion.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct WebSearch {}

/// Configuration for Model Context Protocol (MCP) tools.
///
/// This structure represents MCP-compatible tools that can extend the model's
/// capabilities through standardized protocols. Currently a placeholder for future expansion.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct MCP {}

/// Specifies the format for the model's response.
///
/// This enum controls how the model should structure its output, either as
/// plain text or as a structured JSON object.
///
/// # Variants
///
/// * `Text` - Plain text response format
/// * `JsonObject` - Structured JSON object response format
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type")]
pub enum ResponseFormat {
    /// Plain text response format.
    Text,
    /// Structured JSON object response format.
    JsonObject,
}


#[derive(Debug, Clone)]
pub struct GLM4_5 {}

impl Into<String> for GLM4_5 {
    fn into(self) -> String {
        "glm-4.5".to_string()
    }
}

impl Serialize for GLM4_5 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let model_name: String = self.clone().into();
        serializer.serialize_str(&model_name)
    }
}

impl ModelName for GLM4_5 {}
impl ThinkEnable for GLM4_5 {}

impl Bounded for (GLM4_5, TextMessage) {}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    // 定义测试用的模型类型
    #[derive(Debug, Clone, serde::Serialize)]
    struct TestModel {
        name: String,
    }

    impl TestModel {
        fn new(name: impl Into<String>) -> Self {
            Self { name: name.into() }
        }
    }

    impl Into<String> for TestModel {
        fn into(self) -> String {
            self.name
        }
    }

    impl ModelName for TestModel {}
    impl ThinkEnable for TestModel {}

    // 实现 Bounded trait
    impl Bounded for (TestModel, TextMessage) {}

    #[test]
    fn test_chatbody_basic_serialization() {
        let model = TestModel::new("gpt-4");
        let messages = vec![
            TextMessage::system("You are a helpful assistant."),
            TextMessage::user("Hello, how are you?"),
            TextMessage::assistant("I'm doing well, thank you! How can I help you today?"),
        ];

        let basic_body = ChatBody {
            model: model.clone(),
            messages: messages.clone(),
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
        };

        let json = serde_json::to_string_pretty(&basic_body).unwrap();
        println!("基本 ChatBody 序列化结果:");
        println!("{}\n", json);

        // 验证 JSON 包含必要字段
        assert!(json.contains("\"model\""));
        assert!(json.contains("\"messages\""));
        assert!(json.contains("\"role\": \"system\""));
        assert!(json.contains("\"role\": \"user\""));
        assert!(json.contains("\"role\": \"assistant\""));
    }

    #[test]
    fn test_chatbody_full_serialization() {
        let model = TestModel::new("gpt-4");
        let messages = vec![
            TextMessage::system("You are a helpful assistant."),
            TextMessage::user("Hello!"),
        ];

        let function_call = FunctionCall::new(
            "get_weather",
            "Get current weather information",
            r#"{"type": "object", "properties": {"location": {"type": "string"}}}"#,
        );

        let tools = Tools::FunctionCall {
            function_call: vec![function_call],
        };

        let full_body = ChatBody {
            model: model.clone(),
            messages: messages.clone(),
            request_id: Some("req_123456".to_string()),
            thinking: Some("Let me think about this...".to_string()),
            do_sample: Some(true),
            stream: Some(false),
            temperature: Some(0.7),
            top_p: Some(0.9),
            max_tokens: Some(1000),
            tools: Some(tools),
            user_id: Some("user_789".to_string()),
            stop: Some(vec!["END".to_string()]),
            response_format: Some(ResponseFormat::JsonObject),
        };

        let json = serde_json::to_string_pretty(&full_body).unwrap();
        println!("完整 ChatBody 序列化结果:");
        println!("{}\n", json);

        // 验证所有字段都存在
        assert!(json.contains("\"request_id\": \"req_123456\""));
        assert!(json.contains("\"thinking\": \"Let me think about this...\""));
        assert!(json.contains("\"do_sample\": true"));
        assert!(json.contains("\"stream\": false"));
        assert!(json.contains("\"temperature\": 0.7"));
        assert!(json.contains("\"top_p\": 0.9"));
        assert!(json.contains("\"max_tokens\": 1000"));
        assert!(json.contains("\"tools\""));
        assert!(json.contains("\"user_id\": \"user_789\""));
        assert!(json.contains("\"stop\""));
        assert!(json.contains("\"response_format\""));
    }

    #[test]
    fn test_chatbody_with_tool_calls() {
        let model = TestModel::new("gpt-4");

        let tool_call = ToolCall::new_function(
            "call_123",
            FunctionParams::new("get_weather", r#"{"location": "Beijing"}"#),
        );
        let messages_with_tools = vec![
            TextMessage::system("You are a helpful assistant with access to weather information."),
            TextMessage::user("What's the weather like in Beijing?"),
            TextMessage::assistant_with_tools(
                Some("I'll check the weather in Beijing for you.".to_string()),
                vec![tool_call],
            ),
            TextMessage::tool_with_id("The weather in Beijing is sunny, 25°C", "call_123"),
            TextMessage::assistant(
                "Based on the weather data, it's currently sunny in Beijing with a temperature of 25°C. It's a beautiful day!",
            ),
        ];

        let tools_body = ChatBody {
            model: model.clone(),
            messages: messages_with_tools,
            request_id: Some("req_tools_123".to_string()),
            thinking: None,
            do_sample: Some(false),
            stream: Some(true),
            temperature: Some(0.3),
            top_p: Some(0.8),
            max_tokens: Some(2000),
            tools: Some(Tools::FunctionCall {
                function_call: vec![FunctionCall::new(
                    "get_weather",
                    "Get current weather information for a location",
                    r#"{"type": "object", "properties": {"location": {"type": "string", "description": "The city name"}}}"#,
                )],
            }),
            user_id: Some("user_weather".to_string()),
            stop: None,
            response_format: Some(ResponseFormat::Text),
        };

        let json = serde_json::to_string_pretty(&tools_body).unwrap();
        println!("ChatBody 带工具调用消息序列化结果:");
        println!("{}\n", json);

        // 验证工具调用相关字段
        assert!(json.contains("\"tool_calls\""));
        assert!(json.contains("\"tool_call_id\""));
        assert!(json.contains("\"role\": \"tool\""));
    }

    #[test]
    fn test_chatbody_with_thinking() {
        let model = TestModel::new("gpt-4");

        let thinking_body = ChatBody {
            model: model.clone(),
            messages: vec![TextMessage::user(
                "Solve this complex math problem: 2x + 5 = 15",
            )],
            request_id: Some("req_thinking".to_string()),
            thinking: None,
            do_sample: None,
            stream: None,
            temperature: Some(0.1),
            top_p: None,
            max_tokens: Some(500),
            tools: None,
            user_id: Some("user_math".to_string()),
            stop: None,
            response_format: None,
        }
        .with_thinking("I need to solve for x: 2x + 5 = 15, so 2x = 10, therefore x = 5");

        let json = serde_json::to_string_pretty(&thinking_body).unwrap();
        println!("ChatBody 带 thinking 序列化结果:");
        println!("{}\n", json);

        // 验证 thinking 字段
        assert!(json.contains("\"thinking\""));
        assert!(json.contains("I need to solve for x"));
    }
}
