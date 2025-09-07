//! Chat message types for the model API.
//!
//! This module defines the various message types used in chat conversations,
//! including user messages, assistant responses, system instructions, and tool responses.

use serde::{Deserialize, Serialize};
use validator::*;

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

/// Represents messages in vision-enabled chat conversations.
///
/// This enum defines message types for conversations that can include
/// multimedia content like images, videos, and files alongside text.
/// Each variant is serialized with a "role" field to distinguish message types.
///
/// # Serialization
///
/// Messages are serialized as JSON objects with a "role" field that indicates
/// the message type ("user", "system", or "assistant").
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "role")]
#[serde(rename_all = "lowercase")]
pub enum VisionMessage {
    /// A message from the user/human containing rich multimedia content.
    User { rich_content: RichContent },
    /// A system message that provides instructions or context to the assistant.
    System { content: String },
    /// A response from the AI assistant.
    Assistant { content: Option<String> },
}

/// Represents different types of rich multimedia content in vision messages.
///
/// This enum defines the various types of content that can be included in
/// vision-enabled messages, including text, images, videos, and files.
/// Each variant is serialized with a "type" field to distinguish content types.
///
/// # Serialization
///
/// Content is serialized as JSON objects with a "type" field that indicates
/// the content type ("text", "image_url", "video_url", or "file_url").
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
#[serde(rename_all = "lowercase")]
pub enum RichContent {
    Text {
        text: String,
    },
    /// Image URL or Base64 encoded image data.
    ///
    /// Upload constraints: Each image must be under 5MB with maximum resolution of 6000*6000 pixels.
    /// Supported formats: jpg, png, jpeg.
    ///
    /// Model-specific limits:
    /// - GLM4.5V: maximum 50 images
    /// - GLM-4V-Plus-0111: maximum 5 images
    /// - GLM-4V-Flash: maximum 1 image (Base64 encoding not supported)
    ImageUrl {
        url: String,
    },
    /// Video URL for video content.
    ///
    /// Video size limits:
    /// - GLM-4.5V: maximum 200MB
    /// - GLM-4V-Plus: maximum 20MB, video duration not exceeding 30 seconds
    /// - Other multimodal models: maximum 200MB
    ///
    /// Supported format: mp4
    VideoUrl {
        url: String,
    },
    /// File URL for document content.
    ///
    /// File URL address, Base64 encoding is not supported.
    /// Supported formats: PDF, Word, and other document formats.
    /// Maximum 50 files supported.
    FileUrl {
        url: String,
    },
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

impl serde::Serialize for ToolCall {
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
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;

        // Validation: function field is required when type_ is Function
        if matches!(self.type_, ToolCallType::Function) && self.function.is_none() {
            return Err(serde::ser::Error::custom(
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
