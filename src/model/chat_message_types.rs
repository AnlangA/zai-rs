//! Message types for text, vision and voice chat requests.
//!
//! Model/message compatibility is enforced by the bindings in
//! [`chat_models`]. Constructors in this module shape
//! the JSON payload but generally do not inspect remote media or enforce
//! provider-specific size and duration limits.
//!
//! # Examples
//!
//! ```
//! use zai_rs::model::chat_message_types::*;
//!
//! let text = TextMessage::user("Hello!");
//! let vision = VisionMessage::new_user()
//!     .add_content(VisionRichContent::image("https://example.com/image.jpg"));
//! let voice = VoiceMessage::new_user()
//!     .add_content(VoiceRichContent::input_audio(b"audio_data", VoiceFormat::MP3));
//!
//! let function_params = FunctionParams::new("get_weather", r#"{"location": "Tokyo"}"#);
//! let tool_call = ToolCall::new_function("call_123", function_params);
//! let assistant_msg = TextMessage::assistant_with_tools(None, vec![tool_call]);
//! ```

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64_STANDARD};
use serde::Serialize;

mod tool_call;

pub use tool_call::{FunctionParams, ToolCall, ToolCallType};

/// Text-chat message serialized with a `role` discriminator.
///
/// # Examples
///
/// ```rust
/// # use zai_rs::model::*;
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
        /// The text content of the assistant's response. Optional when tool
        /// calls are present.
        #[serde(skip_serializing_if = "Option::is_none")]
        content: Option<String>,
        /// Tool calls made by the assistant. Empty vector is omitted from
        /// serialization.
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
        /// The ID of the tool call this message is responding to. Optional
        /// field.
        #[serde(skip_serializing_if = "Option::is_none")]
        tool_call_id: Option<String>,
    },
}

impl TextMessage {
    /// Create a user message.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use zai_rs::model::*;
    /// let msg = TextMessage::user("Hello, how can you help me today?");
    /// ```
    pub fn user(content: impl Into<String>) -> Self {
        TextMessage::User {
            content: content.into(),
        }
    }

    /// Create an assistant message containing text and no tool calls.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use zai_rs::model::*;
    /// let msg = TextMessage::assistant("I'm happy to help you with that!");
    /// ```
    pub fn assistant(content: impl Into<String>) -> Self {
        TextMessage::Assistant {
            content: Some(content.into()),
            tool_calls: Vec::new(),
        }
    }

    /// Create an assistant message containing optional text and tool calls.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use zai_rs::model::*;
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

    /// Create a system-instruction message.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use zai_rs::model::*;
    /// let msg = TextMessage::system("You are a helpful assistant specialized in programming.");
    /// ```
    pub fn system(content: impl Into<String>) -> Self {
        TextMessage::System {
            content: content.into(),
        }
    }

    /// Create a tool-result message without a tool-call identifier.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use zai_rs::model::*;
    /// let msg = TextMessage::tool("The current temperature is 22°C");
    /// ```
    pub fn tool(content: impl Into<String>) -> Self {
        TextMessage::Tool {
            content: content.into(),
            tool_call_id: None,
        }
    }

    /// Create a tool-result message linked to `tool_call_id`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use zai_rs::model::*;
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

/// Vision-chat message serialized with a `role` discriminator.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "role")]
#[serde(rename_all = "lowercase")]
pub enum VisionMessage {
    /// A message from the user/human containing rich multimedia content.
    User {
        /// Ordered list of multimedia content parts.
        content: Vec<VisionRichContent>,
    },
    /// A system message that provides instructions or context to the assistant.
    System {
        /// The system-instruction text.
        content: String,
    },
    /// A response from the AI assistant.
    Assistant {
        /// The assistant's reply text, if any.
        content: Option<String>,
    },
}

/// URL descriptor nested inside a `video_url` content part.
#[derive(Debug, Clone, Serialize)]
pub struct VideoUrlInfo {
    /// The URL of the video file.
    pub url: String,
}

/// URL or data-URL descriptor nested inside an `image_url` content part.
#[derive(Debug, Clone, Serialize)]
pub struct ImageUrlInfo {
    /// The URL of the image file.
    pub url: String,
}

/// URL descriptor nested inside a `file_url` content part.
#[derive(Debug, Clone, Serialize)]
pub struct FileUrlInfo {
    /// The URL of the file.
    pub url: String,
}

/// Text or remote media content in a vision user message.
///
/// # Serialization
///
/// Content is serialized as JSON objects with a "type" field that indicates
/// the content type ("text", "image_url", "video_url", or "file_url").
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum VisionRichContent {
    /// Plain text content part.
    Text {
        /// The text value.
        text: String,
    },
    /// Image URL or base64 data URL.
    ///
    /// Media limits vary by model and are enforced by the service, not by this
    /// constructor.
    ImageUrl {
        /// Image URL / base64 descriptor.
        image_url: ImageUrlInfo,
    },
    /// Video URL. Supported formats and size limits vary by model.
    VideoUrl {
        /// Video URL descriptor.
        video_url: VideoUrlInfo,
    },
    /// Document URL; this variant does not accept base64 data.
    FileUrl {
        /// File URL descriptor.
        file_url: FileUrlInfo,
    },
}

impl VisionRichContent {
    /// Create a text content part.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use zai_rs::model::*;
    /// let text = VisionRichContent::text("Hello, world!");
    /// ```
    pub fn text(text: impl Into<String>) -> Self {
        VisionRichContent::Text { text: text.into() }
    }

    /// Create an image content part from a URL or data URL.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use zai_rs::model::*;
    /// let image = VisionRichContent::image("https://example.com/image.jpg");
    /// let base64_image = VisionRichContent::image("data:image/jpeg;base64,/9j/4AAQSkZJRgABAQ...");
    /// ```
    pub fn image(url: impl Into<String>) -> Self {
        VisionRichContent::ImageUrl {
            image_url: ImageUrlInfo { url: url.into() },
        }
    }

    /// Create a video content part from a URL.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use zai_rs::model::*;
    /// let video = VisionRichContent::video("https://example.com/video.mp4");
    /// ```
    pub fn video(url: impl Into<String>) -> Self {
        VisionRichContent::VideoUrl {
            video_url: VideoUrlInfo { url: url.into() },
        }
    }

    /// Create a document content part from a URL.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use zai_rs::model::*;
    /// let file = VisionRichContent::file("https://example.com/document.pdf");
    /// ```
    pub fn file(url: impl Into<String>) -> Self {
        VisionRichContent::FileUrl {
            file_url: FileUrlInfo { url: url.into() },
        }
    }
}

impl VisionMessage {
    /// Create an empty vision user message.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use zai_rs::model::*;
    /// let msg = VisionMessage::new_user();
    /// ```
    pub fn new_user() -> Self {
        VisionMessage::User {
            content: Vec::new(),
        }
    }

    /// Append a content part when this is a user message.
    ///
    /// System and assistant messages are returned unchanged.
    ///
    /// # Examples
    ///
    /// ```
    /// use zai_rs::model::chat_message_types::{VisionMessage, VisionRichContent};
    ///
    /// let image = VisionRichContent::image("https://example.com/image.jpg");
    /// let text = VisionRichContent::text("describe this image");
    /// let msg = VisionMessage::new_user()
    ///     .add_content(image)
    ///     .add_content(text);
    /// ```
    pub fn add_content(self, rich_content: VisionRichContent) -> Self {
        match self {
            VisionMessage::User { mut content } => {
                content.push(rich_content);
                VisionMessage::User { content }
            },
            // Only user messages carry rich content; preserve every other role.
            _ => self,
        }
    }

    /// Create a system-instruction message.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use zai_rs::model::*;
    /// let msg = VisionMessage::system("You are a helpful vision assistant.");
    /// ```
    pub fn system(content: impl Into<String>) -> Self {
        VisionMessage::System {
            content: content.into(),
        }
    }

    /// Create an assistant message containing text.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use zai_rs::model::*;
    /// let msg = VisionMessage::assistant("I can see the image contains a cat.");
    /// ```
    pub fn assistant(content: impl Into<String>) -> Self {
        VisionMessage::Assistant {
            content: Some(content.into()),
        }
    }

    /// Create an assistant message with optional text.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use zai_rs::model::*;
    /// let msg = VisionMessage::assistant_with_content(None);
    /// let msg_with_content = VisionMessage::assistant_with_content(Some("I analyzed the image.".to_string()));
    /// ```
    pub fn assistant_with_content(content: Option<String>) -> Self {
        VisionMessage::Assistant { content }
    }
}

/// Voice-chat message serialized with a `role` discriminator.
///
/// User messages accept text or input audio, system messages accept text, and
/// assistant messages may reference generated audio. The sealed model/message
/// bindings restrict this type to compatible models.
///
/// # Examples
///
/// ```rust
/// # use zai_rs::model::*;
/// let audio_content = VoiceRichContent::input_audio(b"audio_data", VoiceFormat::MP3);
/// let user_msg = VoiceMessage::new_user().add_content(audio_content);
///
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
        content: Vec<VoiceRichContent>,
    },
    /// A system message that provides instructions or context to the assistant.
    System {
        /// The content of the system message (text-only).
        content: String,
    },
    /// A response from the AI assistant, which can include text and/or audio.
    Assistant {
        /// The text content of the assistant's response. Optional when audio is
        /// present.
        #[serde(skip_serializing_if = "Option::is_none")]
        content: Option<String>,
        /// Audio response data generated by the assistant. Optional field.
        #[serde(skip_serializing_if = "Option::is_none")]
        audio: Option<Audio>,
    },
}

/// Text or input-audio content in a voice user message.
///
/// [`input_audio`](Self::input_audio) base64-encodes the supplied bytes. It does
/// not inspect the media duration or verify that the bytes match the selected
/// format.
///
/// # Examples
///
/// ```rust,no_run
/// # use zai_rs::model::*;
/// let text_content = VoiceRichContent::text("Hello, I need help with something.");
///
/// # fn read_audio() -> std::io::Result<()> {
/// let audio_bytes = std::fs::read("audio.mp3")?;
/// let audio_content = VoiceRichContent::input_audio(audio_bytes, VoiceFormat::MP3);
/// # Ok(())
/// # }
///
/// let audio_data = b"raw audio data";
/// let audio_content = VoiceRichContent::input_audio(audio_data, VoiceFormat::WAV);
/// ```
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum VoiceRichContent {
    /// Text content.
    Text {
        /// The text content of the message.
        text: String,
    },
    /// Audio input content, supported only by glm-4-voice model for audio
    /// input.
    ///
    /// # Field Description
    ///
    /// Base64-encoded input audio and its format.
    InputAudio {
        /// Audio data and format information.
        input_audio: InputAudioData,
    },
}

/// Base64-encoded audio supplied in a voice user message.
#[derive(Debug, Clone, Serialize)]
pub struct InputAudioData {
    /// Base64-encoded audio data. Maximum audio duration is 10 minutes.
    /// 1 second of audio = 12.5 tokens, rounded up.
    pub data: String,
    /// Audio format.
    pub format: VoiceFormat,
}

impl VoiceRichContent {
    /// Create a text content part.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use zai_rs::model::*;
    /// let text = VoiceRichContent::text("Hello, world!");
    /// ```
    pub fn text(text: impl Into<String>) -> Self {
        VoiceRichContent::Text { text: text.into() }
    }

    /// Base64-encode `data` and create an input-audio content part.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use zai_rs::model::*;
    /// let audio_bytes = b"audio data";
    /// let audio = VoiceRichContent::input_audio(audio_bytes, VoiceFormat::MP3);
    /// ```
    pub fn input_audio(data: impl AsRef<[u8]>, format: VoiceFormat) -> Self {
        let base64_string = BASE64_STANDARD.encode(data);
        VoiceRichContent::InputAudio {
            input_audio: InputAudioData {
                data: base64_string,
                format,
            },
        }
    }
}

/// Audio format accepted by the voice-chat endpoint.
///
/// # Examples
///
/// ```rust
/// # use zai_rs::model::*;
/// let format = VoiceFormat::MP3;
///
/// let format = VoiceFormat::WAV;
///
/// let format = VoiceFormat::from_extension("mp3").unwrap_or(VoiceFormat::MP3);
/// ```
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum VoiceFormat {
    /// MPEG Audio Layer III.
    MP3,

    /// Waveform Audio File Format.
    WAV,
}

impl VoiceFormat {
    /// Parse a case-insensitive file extension.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use zai_rs::model::*;
    /// assert_eq!(VoiceFormat::from_extension("mp3"), Some(VoiceFormat::MP3));
    /// assert_eq!(VoiceFormat::from_extension("wav"), Some(VoiceFormat::WAV));
    /// assert_eq!(VoiceFormat::from_extension("ogg"), None);
    /// ```
    pub fn from_extension(extension: &str) -> Option<Self> {
        if extension.eq_ignore_ascii_case("mp3") {
            Some(Self::MP3)
        } else if extension.eq_ignore_ascii_case("wav") {
            Some(Self::WAV)
        } else {
            None
        }
    }

    /// Parse a supported MIME type case-insensitively.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use zai_rs::model::*;
    /// assert_eq!(VoiceFormat::from_mime_type("audio/mpeg"), Some(VoiceFormat::MP3));
    /// assert_eq!(VoiceFormat::from_mime_type("audio/wav"), Some(VoiceFormat::WAV));
    /// assert_eq!(VoiceFormat::from_mime_type("audio/ogg"), None);
    /// ```
    pub fn from_mime_type(mime_type: &str) -> Option<Self> {
        if mime_type.eq_ignore_ascii_case("audio/mpeg") {
            Some(Self::MP3)
        } else if mime_type.eq_ignore_ascii_case("audio/wav")
            || mime_type.eq_ignore_ascii_case("audio/x-wav")
        {
            Some(Self::WAV)
        } else {
            None
        }
    }
}

/// Reference to audio previously generated by a voice-capable model.
///
/// # Examples
///
/// ```rust
/// # use zai_rs::model::*;
/// let audio_with_id = Audio::with_id("audio_123");
///
/// let anonymous_audio = Audio::new();
/// ```
#[derive(Debug, Clone, Default, Serialize)]
pub struct Audio {
    /// Provider-issued audio identifier used in conversation history.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
}

impl Audio {
    /// Create an audio reference with a provider-issued identifier.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use zai_rs::model::*;
    /// let audio = Audio::with_id("audio_123");
    /// ```
    pub fn with_id(id: impl Into<String>) -> Self {
        Audio {
            id: Some(id.into()),
        }
    }

    /// Create an empty audio reference.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use zai_rs::model::*;
    /// let audio = Audio::new();
    /// ```
    pub fn new() -> Self {
        Self::default()
    }
}

impl VoiceMessage {
    /// Create an empty voice user message.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use zai_rs::model::*;
    /// let msg = VoiceMessage::new_user();
    /// ```
    pub fn new_user() -> Self {
        VoiceMessage::User {
            content: Vec::new(),
        }
    }

    /// Append a content part when this is a user message.
    ///
    /// System and assistant messages are returned unchanged.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use zai_rs::model::*;
    /// let audio = VoiceRichContent::text("Hello");
    /// let text = VoiceRichContent::text("describe this audio");
    /// let msg = VoiceMessage::new_user()
    ///     .add_content(audio)
    ///     .add_content(text);
    /// ```
    pub fn add_content(self, rich_content: VoiceRichContent) -> Self {
        match self {
            VoiceMessage::User { mut content } => {
                content.push(rich_content);
                VoiceMessage::User { content }
            },
            // Only user messages carry rich content; preserve every other role.
            _ => self,
        }
    }

    /// Create a system-instruction message.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use zai_rs::model::*;
    /// let msg = VoiceMessage::system("You are a helpful voice assistant.");
    /// ```
    pub fn system(content: impl Into<String>) -> Self {
        VoiceMessage::System {
            content: content.into(),
        }
    }

    /// Create an assistant message containing text and no audio.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use zai_rs::model::*;
    /// let msg = VoiceMessage::assistant("I can help you with that!");
    /// ```
    pub fn assistant(content: impl Into<String>) -> Self {
        VoiceMessage::Assistant {
            content: Some(content.into()),
            audio: None,
        }
    }

    /// Create an assistant message containing optional text and audio.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use zai_rs::model::*;
    /// let audio = Audio { id: Some("audio_123".to_string()) };
    /// let msg = VoiceMessage::assistant_with_audio(
    ///     Some("Here's the audio response.".to_string()),
    ///     Some(audio)
    /// );
    /// ```
    pub fn assistant_with_audio(content: Option<String>, audio: Option<Audio>) -> Self {
        VoiceMessage::Assistant { content, audio }
    }

    /// Create an assistant message containing audio and no text.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use zai_rs::model::*;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_message_user() {
        let msg = TextMessage::user("Hello world");
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"role\":\"user\""));
        assert!(json.contains("Hello world"));
    }

    #[test]
    fn test_text_message_assistant() {
        let msg = TextMessage::assistant("I can help");
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"role\":\"assistant\""));
        assert!(json.contains("I can help"));
    }

    #[test]
    fn test_text_message_assistant_with_tools() {
        let func_params = FunctionParams::new("test_func", "{}");
        let tool_call = ToolCall::new_function("call_123", func_params);
        let msg = TextMessage::assistant_with_tools(Some("text".to_string()), vec![tool_call]);

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"role\":\"assistant\""));
        assert!(json.contains("text"));
        assert!(json.contains("call_123"));
    }

    #[test]
    fn test_text_message_assistant_empty_content() {
        let msg = TextMessage::assistant_with_tools(None, vec![]);
        let json = serde_json::to_string(&msg).unwrap();
        // Empty content and empty tool_calls should omit these fields
        assert!(json.contains("\"role\":\"assistant\""));
    }

    #[test]
    fn test_text_message_system() {
        let msg = TextMessage::system("You are helpful");
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"role\":\"system\""));
        assert!(json.contains("You are helpful"));
    }

    #[test]
    fn test_text_message_tool() {
        let msg = TextMessage::tool("Tool result");
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"role\":\"tool\""));
        assert!(json.contains("Tool result"));
    }

    #[test]
    fn test_text_message_tool_with_id() {
        let msg = TextMessage::tool_with_id("Tool result", "call_123");
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"role\":\"tool\""));
        assert!(json.contains("call_123"));
    }

    #[test]
    fn test_vision_message_new_user() {
        let msg = VisionMessage::new_user();
        if let VisionMessage::User { content } = msg {
            assert!(content.is_empty());
        } else {
            panic!("Expected User variant");
        }
    }

    #[test]
    fn test_vision_message_add_content() {
        let msg = VisionMessage::new_user()
            .add_content(VisionRichContent::text("Hello"))
            .add_content(VisionRichContent::image("https://example.com/img.jpg"));

        if let VisionMessage::User { content } = msg {
            assert_eq!(content.len(), 2);
        } else {
            panic!("Expected User variant");
        }
    }

    #[test]
    fn test_vision_message_system() {
        let msg = VisionMessage::system("System instruction");
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"role\":\"system\""));
    }

    #[test]
    fn test_vision_message_assistant() {
        let msg = VisionMessage::assistant("I see a cat");
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"role\":\"assistant\""));
    }

    #[test]
    fn test_vision_rich_content_text() {
        let content = VisionRichContent::text("Hello");
        let json = serde_json::to_string(&content).unwrap();
        assert!(json.contains("\"type\":\"text\""));
        assert!(json.contains("Hello"));
    }

    #[test]
    fn test_vision_rich_content_image() {
        let content = VisionRichContent::image("https://example.com/img.jpg");
        let json = serde_json::to_string(&content).unwrap();
        assert!(json.contains("\"type\":\"image_url\""));
        assert!(json.contains("https://example.com/img.jpg"));
    }

    #[test]
    fn test_vision_rich_content_video() {
        let content = VisionRichContent::video("https://example.com/video.mp4");
        let json = serde_json::to_string(&content).unwrap();
        assert!(json.contains("\"type\":\"video_url\""));
    }

    #[test]
    fn test_vision_rich_content_file() {
        let content = VisionRichContent::file("https://example.com/doc.pdf");
        let json = serde_json::to_string(&content).unwrap();
        assert!(json.contains("\"type\":\"file_url\""));
    }

    #[test]
    fn test_voice_message_new_user() {
        let msg = VoiceMessage::new_user();
        if let VoiceMessage::User { content } = msg {
            assert!(content.is_empty());
        } else {
            panic!("Expected User variant");
        }
    }

    #[test]
    fn test_voice_message_add_content() {
        let msg = VoiceMessage::new_user()
            .add_content(VoiceRichContent::text("Hello"))
            .add_content(VoiceRichContent::input_audio(
                b"audio_data",
                VoiceFormat::MP3,
            ));

        if let VoiceMessage::User { content } = msg {
            assert_eq!(content.len(), 2);
        } else {
            panic!("Expected User variant");
        }
    }

    #[test]
    fn test_voice_message_system() {
        let msg = VoiceMessage::system("System instruction");
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"role\":\"system\""));
    }

    #[test]
    fn test_voice_message_assistant() {
        let msg = VoiceMessage::assistant("Audio response");
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"role\":\"assistant\""));
    }

    #[test]
    fn test_voice_message_assistant_with_audio() {
        let audio = Audio::with_id("audio_123");
        let msg = VoiceMessage::assistant_with_audio(Some("text".to_string()), Some(audio));
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"role\":\"assistant\""));
        assert!(json.contains("audio_123"));
    }

    #[test]
    fn test_voice_message_assistant_audio_only() {
        let audio = Audio::with_id("audio_123");
        let msg = VoiceMessage::assistant_audio_only(audio);
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"role\":\"assistant\""));
        assert!(json.contains("audio_123"));
    }

    #[test]
    fn test_voice_rich_content_text() {
        let content = VoiceRichContent::text("Hello");
        let json = serde_json::to_string(&content).unwrap();
        assert!(json.contains("\"type\":\"text\""));
        assert!(json.contains("Hello"));
    }

    #[test]
    fn test_voice_rich_content_input_audio() {
        let content = VoiceRichContent::input_audio(b"audio_bytes", VoiceFormat::MP3);
        let json = serde_json::to_string(&content).unwrap();
        assert!(json.contains("\"type\":\"input_audio\""));
        assert!(json.contains("\"format\":\"mp3\""));
        // Should contain base64 encoded data
        assert!(json.contains("\"data\":"));
    }

    #[test]
    fn test_voice_format_from_extension() {
        assert_eq!(VoiceFormat::from_extension("mp3"), Some(VoiceFormat::MP3));
        assert_eq!(VoiceFormat::from_extension("MP3"), Some(VoiceFormat::MP3));
        assert_eq!(VoiceFormat::from_extension("wav"), Some(VoiceFormat::WAV));
        assert_eq!(VoiceFormat::from_extension("WAV"), Some(VoiceFormat::WAV));
        assert_eq!(VoiceFormat::from_extension("ogg"), None);
        assert_eq!(VoiceFormat::from_extension("flac"), None);
    }

    #[test]
    fn test_voice_format_from_mime_type() {
        assert_eq!(
            VoiceFormat::from_mime_type("audio/mpeg"),
            Some(VoiceFormat::MP3)
        );
        assert_eq!(
            VoiceFormat::from_mime_type("audio/wav"),
            Some(VoiceFormat::WAV)
        );
        assert_eq!(
            VoiceFormat::from_mime_type("audio/x-wav"),
            Some(VoiceFormat::WAV)
        );
        assert_eq!(VoiceFormat::from_mime_type("audio/ogg"), None);
        assert_eq!(VoiceFormat::from_mime_type("audio/flac"), None);
    }

    #[test]
    fn test_audio_new() {
        let audio = Audio::new();
        assert!(audio.id.is_none());
    }

    #[test]
    fn test_audio_with_id() {
        let audio = Audio::with_id("audio_123");
        assert_eq!(audio.id, Some("audio_123".to_string()));
    }

    #[test]
    fn test_audio_serialization() {
        let audio = Audio::with_id("audio_123");
        let json = serde_json::to_string(&audio).unwrap();
        assert!(json.contains("\"audio_123\""));

        let audio_no_id = Audio::new();
        let json_no_id = serde_json::to_string(&audio_no_id).unwrap();
        // ID field should be omitted when None
        assert!(!json_no_id.contains("id"));
    }

    #[test]
    fn test_tool_call_new_function() {
        let func_params = FunctionParams::new("test_func", r#"{"arg":"value"}"#);
        let tool_call = ToolCall::new_function("call_123", func_params);
        let json = serde_json::to_string(&tool_call).unwrap();
        assert!(json.contains("\"id\":\"call_123\""));
        assert!(json.contains("\"type\":\"function\""));
        assert!(json.contains("test_func"));
    }

    #[test]
    fn test_tool_call_new_web_search() {
        let tool_call = ToolCall::new_web_search("search_456");
        let json = serde_json::to_string(&tool_call).unwrap();
        assert!(json.contains("\"id\":\"search_456\""));
        assert!(json.contains("\"type\":\"web_search\""));
    }

    #[test]
    fn test_tool_call_new_retrieval() {
        let tool_call = ToolCall::new_retrieval("retrieval_789");
        let json = serde_json::to_string(&tool_call).unwrap();
        assert!(json.contains("\"id\":\"retrieval_789\""));
        assert!(json.contains("\"type\":\"retrieval\""));
    }

    #[test]
    fn test_tool_call_function_without_params_is_rejected() {
        let tool_call = ToolCall {
            id: "call_123".to_string(),
            type_: ToolCallType::Function,
            function: None,
        };
        let result = serde_json::to_string(&tool_call);
        assert!(result.is_err());
    }

    #[test]
    fn test_function_params_new() {
        let params = FunctionParams::new("test_func", r#"{"arg":"value"}"#);
        assert_eq!(params.name, "test_func");
        assert_eq!(params.arguments, r#"{"arg":"value"}"#);
    }

    #[test]
    fn test_function_params_serialization() {
        let params = FunctionParams::new("test_func", r#"{"arg":"value"}"#);
        let json = serde_json::to_string(&params).unwrap();
        assert!(json.contains("\"name\":\"test_func\""));
        // The wire contract preserves `arguments` as a JSON string rather than
        // embedding it as an object.
        assert!(json.contains(r#""arguments":"{\"arg\":\"value\"}""#));
    }

    #[test]
    fn test_function_params_deserialization() {
        let json = r#"{"name":"test_func","arguments":"{\"arg\":\"value\"}"}"#;
        let params: FunctionParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.name, "test_func");
        assert_eq!(params.arguments, r#"{"arg":"value"}"#);
    }
}
