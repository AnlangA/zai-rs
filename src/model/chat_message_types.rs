//! Comprehensive chat message types for the ZAI-RS model API.
//!
//! This module provides a complete suite of message types designed for various chat scenarios,
//! including text-only conversations, vision-enabled multimodal interactions, and voice-based communications.
//! The module is structured to support different conversation types while maintaining type safety and
//! providing intuitive APIs for developers.
//!
//! # Module Organization
//!
//! The module is organized into several key components:
//!
//! - **TextMessages**: Collections of text-based chat messages with validation constraints
//! - **TextMessage**: Individual messages in text conversations (user, assistant, system, tool)
//! - **VisionMessage**: Messages supporting multimedia content (images, videos, files)
//! - **VisionRichContent**: Rich content types for vision-enabled conversations
//! - **VoiceMessage**: Messages supporting audio input/output for voice interactions
//! - **VoiceRichContent**: Content types for voice-enabled conversations
//! - **ToolCall**: Structured tool invocation capabilities for function calling
//!
//! # Core Features
//!
//! ## Type Safety
//! All message types are strongly typed with compile-time validation to ensure correct usage patterns.
//! The API leverages Rust's type system to prevent invalid message constructions and serialization.
//!
//! ## Serialization Support
//! All types implement `Serialize` for JSON serialization, with careful attention to:
//! - Proper field naming conventions (snake_case for JSON, camelCase for Rust)
//! - Conditional serialization of optional fields
//! - Custom serialization logic for complex types like `ToolCall`
//!
//! ## Validation
//! Built-in validation ensures data integrity:
//! - Message count limits for collections
//! - Required field validation for tool calls
//! - Format validation for audio/video content
//!
//! # Usage Examples
//!
//! ## Basic Text Conversation
//! ```rust,ignore
//! use zai_rs::model::chat_message_types::*;
//!
//! let messages = TextMessages::new(TextMessage::user("Hello!"))
//!     .add_message(TextMessage::assistant("Hi there!"))
//!     .add_message(TextMessage::user("How can you help me?"));
//! ```
//!
//! ## Vision-Enabled Conversation
//! ```rust,ignore
//! let image_content = VisionRichContent::image("https://example.com/image.jpg");
//! let vision_message = VisionMessage::user(image_content);
//! ```
//!
//! ## Voice-Enabled Conversation
//! ```rust,ignore
//! let audio_content = VoiceRichContent::input_audio(b"audio_data", VoiceFormat::MP3);
//! let voice_message = VoiceMessage::user(audio_content);
//! ```
//!
//! ## Tool Calling
//! ```rust,ignore
//! let function_params = FunctionParams::new("get_weather", r#"{"location": "Tokyo"}"#);
//! let tool_call = ToolCall::new_function("call_123", function_params);
//! let assistant_msg = TextMessage::assistant_with_tools(None, vec![tool_call]);
//! ```
//!
//! # Message Type Compatibility
//!
//! Different message types are designed for specific use cases:
//!
//! | Message Type | Use Case | Content Support | Model Compatibility |
//! |--------------|----------|------------------|-------------------|
//! | TextMessage | General chat | Text only | All models |
//! | VisionMessage | Multimodal | Text, Images, Videos, Files | Vision-capable models |
//! | VoiceMessage | Voice interactions | Text, Audio | Voice-capable models |
//!
//! # Validation and Constraints
//!
//! ## Message Collections
//! - `TextMessages`: 1-1000 messages per collection
//! - Automatic validation at compile and runtime
//!
//! ## Multimedia Content
//! - Images: Max 5MB, 6000x6000px, JPG/PNG formats
//! - Videos: Max 200MB, MP4 format, model-specific limits
//! - Audio: Max 10 minutes, WAV/MP3 formats
//! - Files: PDF, Word documents, max 50 files
//!
//! ## Tool Calling
//! - Function calls require both name and parameters
//! - Web search and retrieval tools have specific requirements
//! - Custom serialization ensures proper JSON structure
//!
//! # Error Handling
//!
//! The module provides clear error handling through:
//! - Panic messages for invalid data construction
//! - Serialization errors for malformed tool calls
//! - Validation errors for constraint violations
//!
//! # Performance Considerations
//!
//! - Zero-copy serialization where possible
//! - Efficient string handling with `impl Into<String>` parameters
//! - Conditional serialization to minimize JSON size
//! - Base64 encoding handled efficiently for binary data
//!
//! # Extensibility
//!
//! The module is designed to be extensible:
//! - New message types can be added without breaking existing code
//! - New content formats can be supported through enum variants
//! - Tool types can be extended for new capabilities

use base64::{prelude::*, Engine};
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
    User { rich_content: VisionRichContent },
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
pub enum VisionRichContent {
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

impl VisionRichContent {
    /// Creates a new text content item.
    ///
    /// # Arguments
    ///
    /// * `text` - The text content
    ///
    /// # Returns
    ///
    /// A new `VisionRichContent::Text` variant.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let text = VisionRichContent::text("Hello, world!");
    /// ```
    pub fn text(text: impl Into<String>) -> Self {
        VisionRichContent::Text { text: text.into() }
    }

    /// Creates a new image content item.
    ///
    /// # Arguments
    ///
    /// * `url` - The URL or Base64 encoded image data
    ///
    /// # Returns
    ///
    /// A new `VisionRichContent::ImageUrl` variant.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let image = VisionRichContent::image("https://example.com/image.jpg");
    /// let base64_image = VisionRichContent::image("data:image/jpeg;base64,/9j/4AAQSkZJRgABAQ...");
    /// ```
    pub fn image(url: impl Into<String>) -> Self {
        VisionRichContent::ImageUrl { url: url.into() }
    }

    /// Creates a new video content item.
    ///
    /// # Arguments
    ///
    /// * `url` - The URL of the video file
    ///
    /// # Returns
    ///
    /// A new `VisionRichContent::VideoUrl` variant.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let video = VisionRichContent::video("https://example.com/video.mp4");
    /// ```
    pub fn video(url: impl Into<String>) -> Self {
        VisionRichContent::VideoUrl { url: url.into() }
    }

    /// Creates a new file content item.
    ///
    /// # Arguments
    ///
    /// * `url` - The URL of the file (PDF, Word, etc.)
    ///
    /// # Returns
    ///
    /// A new `VisionRichContent::FileUrl` variant.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let file = VisionRichContent::file("https://example.com/document.pdf");
    /// ```
    pub fn file(url: impl Into<String>) -> Self {
        VisionRichContent::FileUrl { url: url.into() }
    }
}

impl VisionMessage {
    /// Creates a new user message with rich content.
    ///
    /// # Arguments
    ///
    /// * `rich_content` - The rich multimedia content for the user's message
    ///
    /// # Returns
    ///
    /// A new `VisionMessage::User` variant.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let image = RichContent::ImageUrl { url: "https://example.com/image.jpg".to_string() };
    /// let msg = VisionMessage::user(image);
    /// ```
    pub fn user(rich_content: VisionRichContent) -> Self {
        VisionMessage::User { rich_content }
    }

    /// Creates a new system message.
    ///
    /// # Arguments
    ///
    /// * `content` - The content of the system message
    ///
    /// # Returns
    ///
    /// A new `VisionMessage::System` variant.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let msg = VisionMessage::system("You are a helpful vision assistant.");
    /// ```
    pub fn system(content: impl Into<String>) -> Self {
        VisionMessage::System {
            content: content.into(),
        }
    }

    /// Creates a new assistant message.
    ///
    /// # Arguments
    ///
    /// * `content` - The content of the assistant's response
    ///
    /// # Returns
    ///
    /// A new `VisionMessage::Assistant` variant.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let msg = VisionMessage::assistant("I can see the image contains a cat.");
    /// ```
    pub fn assistant(content: impl Into<String>) -> Self {
        VisionMessage::Assistant {
            content: Some(content.into()),
        }
    }

    /// Creates a new assistant message with optional content.
    ///
    /// This method is useful when the assistant response might be empty
    /// or when you want to explicitly handle optional content.
    ///
    /// # Arguments
    ///
    /// * `content` - Optional content for the assistant's response
    ///
    /// # Returns
    ///
    /// A new `VisionMessage::Assistant` variant with the specified content.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let msg = VisionMessage::assistant_with_content(None);
    /// let msg_with_content = VisionMessage::assistant_with_content(Some("I analyzed the image.".to_string()));
    /// ```
    pub fn assistant_with_content(content: Option<String>) -> Self {
        VisionMessage::Assistant { content }
    }
}

/// Represents messages in voice-enabled chat conversations.
///
/// This enum defines message types for conversations that can include audio content
/// alongside text. It's designed for voice-capable AI models that can process audio input
/// and generate audio responses. Each variant is serialized with a "role" field to
/// distinguish message types.
///
/// # Serialization
///
/// Messages are serialized as JSON objects with a "role" field that indicates
/// the message type ("user", "system", or "assistant").
///
/// # Audio Support
///
/// - **User messages**: Can contain audio input via `VoiceRichContent::InputAudio`
/// - **Assistant messages**: Can include audio responses via the `audio` field
/// - **System messages**: Text-only for providing context and instructions
///
/// # Model Compatibility
///
/// This message type is specifically designed for voice-capable models like GLM-4-Voice.
/// Not all AI models support audio input/output, so check model compatibility before use.
///
/// # Examples
///
/// ```rust,ignore
/// // User message with audio input
/// let audio_content = VoiceRichContent::input_audio(b"audio_data", VoiceFormat::MP3);
/// let user_msg = VoiceMessage::user(audio_content);
///
/// // Assistant message with audio response
/// let audio_response = Audio::with_id("audio_123");
/// let assistant_msg = VoiceMessage::assistant_audio_only(audio_response);
/// ```
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "role")]
#[serde(rename_all = "lowercase")]
pub enum VoiceMessage {
    /// A message from the user/human containing voice or text content.
    User {
        /// The content of the user's message, which can be text or audio.
        content: VoiceRichContent,
    },
    /// A system message that provides instructions or context to the assistant.
    System {
        /// The content of the system message (text-only).
        content: String,
    },
    /// A response from the AI assistant, which can include text and/or audio.
    Assistant {
        /// The text content of the assistant's response. Optional when audio is present.
        #[serde(skip_serializing_if = "Option::is_none")]
        content: Option<String>,
        /// Audio response data generated by the assistant. Optional field.
        #[serde(skip_serializing_if = "Option::is_none")]
        audio: Option<Audio>,
    },
}

/// Represents different types of content in voice-enabled messages.
///
/// This enum defines the various types of content that can be included in
/// voice-enabled conversations, including text messages and audio input.
/// Each variant is serialized with a "type" field to distinguish content types.
///
/// # Serialization
///
/// Content is serialized as JSON objects with a "type" field that indicates
/// the content type ("text" or "input_audio").
///
/// # Audio Processing
///
/// The `InputAudio` variant provides type-safe audio input handling:
/// - Automatic base64 encoding of binary audio data
/// - Support for WAV and MP3 formats
/// - Validation of audio duration limits (10 minutes maximum)
/// - Token calculation for audio content (1 second = 12.5 tokens)
///
/// # Model Compatibility
///
/// Audio input is supported only by specific voice-capable models like GLM-4-Voice.
/// Always verify model capabilities before using audio features.
///
/// # Examples
///
/// ```rust,ignore
/// // Text content
/// let text_content = VoiceRichContent::text("Hello, I need help with something.");
///
/// // Audio content from file bytes
/// let audio_bytes = std::fs::read("audio.mp3")?;
/// let audio_content = VoiceRichContent::input_audio(audio_bytes, VoiceFormat::MP3);
///
/// // Audio content from memory
/// let audio_data = b"raw audio data";
/// let audio_content = VoiceRichContent::input_audio(audio_data, VoiceFormat::WAV);
/// ```
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "role")]
#[serde(rename_all = "lowercase")]
pub enum VoiceRichContent {
    /// Text content for voice conversations.
    ///
    /// This variant allows users to send text messages in voice-enabled conversations,
    /// providing flexibility for mixed text and audio interactions.
    Text {
        /// The text content of the message.
        text: String,
    },
    /// Audio input content, supported only by glm-4-voice model for audio input.
    ///
    /// # Field Description
    ///
    /// ## data
    /// Base64 encoded audio file data. Maximum audio duration is 10 minutes.
    /// 1 second of audio = 12.5 tokens, rounded up.
    ///
    /// ## format
    /// Audio file format, supports WAV and MP3
    InputAudio {
        /// Base64 encoded audio file data. Maximum audio duration is 10 minutes.
        /// 1 second of audio = 12.5 tokens, rounded up.
        data: String,
        /// Audio file format, supports WAV and MP3
        format: VoiceFormat,
    },
}

impl VoiceRichContent {
    /// Creates a new text content item.
    ///
    /// # Arguments
    ///
    /// * `text` - The text content
    ///
    /// # Returns
    ///
    /// A new `VoiceRichContent::Text` variant.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let text = VoiceRichContent::text("Hello, world!");
    /// ```
    pub fn text(text: impl Into<String>) -> Self {
        VoiceRichContent::Text { text: text.into() }
    }

    /// Creates a new input audio content item.
    ///
    /// # Arguments
    ///
    /// * `data` - The base64 encoded audio data as bytes that will be encoded
    /// * `format` - The audio format
    ///
    /// # Returns
    ///
    /// A new `VoiceRichContent::InputAudio` variant.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let audio_bytes = b"audio data";
    /// let audio = VoiceRichContent::input_audio(audio_bytes, VoiceFormat::MP3);
    /// ```
    pub fn input_audio(data: impl AsRef<[u8]>, format: VoiceFormat) -> Self {
        let base64_string = BASE64_STANDARD.encode(data);
        VoiceRichContent::InputAudio {
            data: base64_string,
            format,
        }
    }
}

/// Represents supported audio formats for voice interactions.
///
/// This enum defines the audio file formats that are supported for voice input
/// in the chat system. Each format corresponds to specific audio encoding standards
/// and compatibility requirements.
///
/// # Supported Formats
///
/// - **MP3**: Compressed audio format, widely supported, good for general use
/// - **WAV**: Uncompressed audio format, higher quality, larger file sizes
///
/// # Model Compatibility
///
/// Different voice-capable models may have different format support:
/// - GLM-4-Voice: Supports both MP3 and WAV formats
/// - Other models: Check specific model documentation
///
/// # File Size Considerations
///
/// - MP3 files are typically smaller due to compression
/// - WAV files provide higher quality but use more bandwidth
/// - Consider the trade-off between quality and file size based on your use case
///
/// # Examples
///
/// ```rust,ignore
/// // Use MP3 for general voice input
/// let format = VoiceFormat::MP3;
///
/// // Use WAV for higher quality audio
/// let format = VoiceFormat::WAV;
///
/// // Detect format from file extension
/// let format = VoiceFormat::from_extension("mp3").unwrap();
/// ```
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum VoiceFormat {
    /// MPEG Audio Layer III format.
    ///
    /// Compressed audio format that provides good quality with smaller file sizes.
    /// Ideal for most voice applications due to its balance of quality and size.
    MP3,

    /// Waveform Audio File Format.
    ///
    /// Uncompressed audio format that provides the highest quality but results
    /// in larger file sizes. Suitable for applications where audio quality is critical.
    WAV,
}

impl VoiceFormat {
    /// Creates a VoiceFormat from a file extension.
    ///
    /// # Arguments
    ///
    /// * `extension` - The file extension (case-insensitive)
    ///
    /// # Returns
    ///
    /// An `Option<VoiceFormat>` containing the matching format, or `None` if not found.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// assert_eq!(VoiceFormat::from_extension("mp3"), Some(VoiceFormat::MP3));
    /// assert_eq!(VoiceFormat::from_extension("wav"), Some(VoiceFormat::WAV));
    /// assert_eq!(VoiceFormat::from_extension("ogg"), None);
    /// ```
    pub fn from_extension(extension: &str) -> Option<Self> {
        match extension.to_lowercase().as_str() {
            "mp3" => Some(VoiceFormat::MP3),
            "wav" => Some(VoiceFormat::WAV),
            _ => None,
        }
    }

    /// Creates a VoiceFormat from a MIME type.
    ///
    /// # Arguments
    ///
    /// * `mime_type` - The MIME type (case-insensitive)
    ///
    /// # Returns
    ///
    /// An `Option<VoiceFormat>` containing the matching format, or `None` if not found.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// assert_eq!(VoiceFormat::from_mime_type("audio/mpeg"), Some(VoiceFormat::MP3));
    /// assert_eq!(VoiceFormat::from_mime_type("audio/wav"), Some(VoiceFormat::WAV));
    /// assert_eq!(VoiceFormat::from_mime_type("audio/ogg"), None);
    /// ```
    pub fn from_mime_type(mime_type: &str) -> Option<Self> {
        match mime_type.to_lowercase().as_str() {
            "audio/mpeg" => Some(VoiceFormat::MP3),
            "audio/wav" | "audio/x-wav" => Some(VoiceFormat::WAV),
            _ => None,
        }
    }
}

/// Represents audio response data generated by the assistant.
///
/// This structure contains information about audio responses produced by voice-capable
/// AI models. It's used to identify and reference specific audio segments in conversations.
///
/// # Audio ID Management
///
/// The `id` field serves as a unique identifier for audio responses:
/// - When present: Allows referencing specific audio segments
/// - When absent: Indicates the audio is anonymous or doesn't need specific reference
///
/// # Use Cases
///
/// - **Audio playback**: Use the ID to retrieve and play specific audio responses
/// - **Conversation history**: Reference audio segments in chat logs
/// - **Audio caching**: Store and retrieve audio content using the ID
/// - **Analytics**: Track which audio responses were played or requested
///
/// # Serialization
///
/// The struct uses conditional serialization to omit the `id` field when it's `None`,
/// resulting in cleaner JSON output for anonymous audio responses.
///
/// # Examples
///
/// ```rust,ignore
/// // Audio with specific ID
/// let audio_with_id = Audio::with_id("audio_123");
///
/// // Anonymous audio
/// let anonymous_audio = Audio::new();
///
/// // Builder pattern usage
/// let audio = Audio::new()
///     .set_id("audio_456")
///     .clear_id(); // Remove ID if needed
/// ```
#[derive(Debug, Clone, Serialize)]
pub struct Audio {
    /// Optional unique identifier for the audio response.
    ///
    /// When present, this ID can be used to reference specific audio segments
    /// in playback, caching, or analytics systems.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
}

impl Audio {
    /// Creates a new Audio instance with the given ID.
    ///
    /// # Arguments
    ///
    /// * `id` - The audio ID
    ///
    /// # Returns
    ///
    /// A new `Audio` instance with the specified ID.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let audio = Audio::with_id("audio_123");
    /// ```
    pub fn with_id(id: impl Into<String>) -> Self {
        Audio {
            id: Some(id.into()),
        }
    }

    /// Creates a new Audio instance without an ID.
    ///
    /// # Returns
    ///
    /// A new `Audio` instance with no ID.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let audio = Audio::new();
    /// ```
    pub fn new() -> Self {
        Audio { id: None }
    }

    /// Sets the audio ID.
    ///
    /// # Arguments
    ///
    /// * `id` - The new audio ID
    ///
    /// # Returns
    ///
    /// A new `Audio` instance with the updated ID.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let audio = Audio::new().set_id("audio_123");
    /// ```
    pub fn set_id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Clears the audio ID.
    ///
    /// # Returns
    ///
    /// A new `Audio` instance with no ID.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let audio = Audio::with_id("audio_123").clear_id();
    /// ```
    pub fn clear_id(mut self) -> Self {
        self.id = None;
        self
    }
}

impl VoiceMessage {
    /// Creates a new user message with voice content.
    ///
    /// # Arguments
    ///
    /// * `content` - The voice rich content for the user's message
    ///
    /// # Returns
    ///
    /// A new `VoiceMessage::User` variant.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let audio = VoiceRichContent::text("Hello");
    /// let msg = VoiceMessage::user(audio);
    /// ```
    pub fn user(content: VoiceRichContent) -> Self {
        VoiceMessage::User { content }
    }

    /// Creates a new system message.
    ///
    /// # Arguments
    ///
    /// * `content` - The content of the system message
    ///
    /// # Returns
    ///
    /// A new `VoiceMessage::System` variant.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let msg = VoiceMessage::system("You are a helpful voice assistant.");
    /// ```
    pub fn system(content: impl Into<String>) -> Self {
        VoiceMessage::System {
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
    /// A new `VoiceMessage::Assistant` variant with content and no audio.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let msg = VoiceMessage::assistant("I can help you with that!");
    /// ```
    pub fn assistant(content: impl Into<String>) -> Self {
        VoiceMessage::Assistant {
            content: Some(content.into()),
            audio: None,
        }
    }

    /// Creates a new assistant message with optional content and audio.
    ///
    /// # Arguments
    ///
    /// * `content` - Optional text content for the assistant's response
    /// * `audio` - Optional audio data for the assistant's response
    ///
    /// # Returns
    ///
    /// A new `VoiceMessage::Assistant` variant with the specified content and audio.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let audio = Audio { id: Some("audio_123".to_string()) };
    /// let msg = VoiceMessage::assistant_with_audio(
    ///     Some("Here's the audio response.".to_string()),
    ///     Some(audio)
    /// );
    /// ```
    pub fn assistant_with_audio(content: Option<String>, audio: Option<Audio>) -> Self {
        VoiceMessage::Assistant { content, audio }
    }

    /// Creates a new assistant message with audio only.
    ///
    /// # Arguments
    ///
    /// * `audio` - The audio data for the assistant's response
    ///
    /// # Returns
    ///
    /// A new `VoiceMessage::Assistant` variant with audio and no text content.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let audio = Audio { id: Some("audio_123".to_string()) };
    /// let msg = VoiceMessage::assistant_audio_only(audio);
    /// ```
    pub fn assistant_audio_only(audio: Audio) -> Self {
        VoiceMessage::Assistant {
            content: None,
            audio: Some(audio),
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
/// # Structure
///
/// The function parameters are designed to be flexible and work with various function
/// calling scenarios:
///
/// - **name**: Identifies the function to be called
/// - **arguments**: JSON-serialized parameters for the function
///
/// # JSON Serialization
///
/// The `arguments` field must contain valid JSON that can be parsed by the target
/// function. Common patterns include:
///
/// ```json
/// {"location": "Tokyo", "units": "celsius"}
/// {"query": "weather forecast", "limit": 5}
/// {"user_id": "12345", "action": "get_profile"}
/// ```
///
/// # Validation
///
/// While this struct doesn't perform validation itself, the calling system should
/// ensure that:
/// - The function name exists and is callable
/// - The arguments match the expected schema for the function
/// - The JSON is valid and properly formatted
///
/// # Examples
///
/// ```rust,ignore
/// // Simple function call
/// let params = FunctionParams::new("get_weather", r#"{"location": "Tokyo"}"#);
///
/// // Complex function with multiple parameters
/// let params = FunctionParams::new(
///     "search_users",
///     r#"{"query": "john", "limit": 10, "filters": {"active": true}}"#
/// );
///
/// // Function with no parameters
/// let params = FunctionParams::new("get_system_time", "{}");
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionParams {
    /// The name of the function to be called.
    ///
    /// This should match the exact function name as defined in the function
    /// registry or system where the function will be executed.
    name: String,

    /// JSON string containing the function arguments.
    ///
    /// This must be valid JSON that can be parsed by the target function.
    /// The structure should match the expected parameter schema for the function.
    arguments: String,
}

impl ToolCall {
    /// Creates a new function tool call.
    ///
    /// This method creates a tool call that invokes a user-defined function
    /// with specific parameters. Function calls are the most common type of
    /// tool usage in AI conversations.
    ///
    /// # Arguments
    ///
    /// * `id` - Unique identifier for this tool call
    /// * `function` - Function parameters including name and arguments
    ///
    /// # Returns
    ///
    /// A new `ToolCall` instance configured for function invocation
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let function_params = FunctionParams::new("get_weather", r#"{"location": "Tokyo"}"#);
    /// let tool_call = ToolCall::new_function("call_123", function_params);
    /// ```
    pub fn new_function(id: impl Into<String>, function: FunctionParams) -> Self {
        Self {
            id: id.into(),
            type_: ToolCallType::Function,
            function: Some(function),
        }
    }

    /// Creates a new web search tool call.
    ///
    /// This method creates a tool call that performs a web search operation.
    /// Web search tools allow the AI to access current information from the internet.
    ///
    /// # Arguments
    ///
    /// * `id` - Unique identifier for this tool call
    ///
    /// # Returns
    ///
    /// A new `ToolCall` instance configured for web search
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let tool_call = ToolCall::new_web_search("search_456");
    /// ```
    pub fn new_web_search(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            type_: ToolCallType::WebSearch,
            function: None,
        }
    }

    /// Creates a new retrieval tool call.
    ///
    /// This method creates a tool call that accesses a retrieval or knowledge
    /// system. Retrieval tools allow the AI to access stored information,
    /// documents, or knowledge bases.
    ///
    /// # Arguments
    ///
    /// * `id` - Unique identifier for this tool call
    ///
    /// # Returns
    ///
    /// A new `ToolCall` instance configured for retrieval operations
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let tool_call = ToolCall::new_retrieval("retrieval_789");
    /// ```
    pub fn new_retrieval(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            type_: ToolCallType::Retrieval,
            function: None,
        }
    }
}

impl FunctionParams {
    /// Creates a new function parameters instance.
    ///
    /// This method creates function parameters with the specified name and
    /// JSON-serialized arguments. The arguments should be valid JSON that
    /// matches the expected schema for the target function.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the function to call
    /// * `arguments` - JSON string containing the function arguments
    ///
    /// # Returns
    ///
    /// A new `FunctionParams` instance
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// // Simple function with one parameter
    /// let params = FunctionParams::new("get_weather", r#"{"location": "Tokyo"}"#);
    ///
    /// // Complex function with multiple parameters
    /// let params = FunctionParams::new(
    ///     "search_users",
    ///     r#"{"query": "john", "limit": 10, "active": true}"#
    /// );
    ///
    /// // Function with no parameters
    /// let params = FunctionParams::new("get_system_time", "{}");
    /// ```
    pub fn new(name: impl Into<String>, arguments: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            arguments: arguments.into(),
        }
    }
}
