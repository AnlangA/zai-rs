//! Chat-related data models

use serde::{Deserialize, Serialize};
use validator::Validate;
use zai_rs::model::{TextMessage, ThinkingType, chat_models::GLM4_6};

/// Chat request payload
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct ChatRequest {
    /// User's message
    #[validate(length(
        min = 1,
        max = 10000,
        message = "Message must be between 1 and 10000 characters"
    ))]
    pub message: String,

    /// Session ID for conversation continuity
    pub session_id: Option<String>,

    /// Enable think mode for enhanced reasoning
    pub think: Option<bool>,

    /// Model to use (optional, defaults to GLM4_6)
    pub model: Option<String>,

    /// Temperature for response generation (0.0 - 2.0)
    #[validate(range(
        min = 0.0,
        max = 2.0,
        message = "Temperature must be between 0.0 and 2.0"
    ))]
    pub temperature: Option<f32>,

    /// Top-p sampling parameter (0.0 - 1.0)
    #[validate(range(min = 0.0, max = 1.0, message = "Top-p must be between 0.0 and 1.0"))]
    pub top_p: Option<f32>,

    /// Maximum tokens to generate
    #[validate(range(min = 1, max = 8192, message = "Max tokens must be between 1 and 8192"))]
    pub max_tokens: Option<u32>,

    /// Enable streaming response
    pub stream: Option<bool>,

    /// System prompt override
    pub system_prompt: Option<String>,
}

impl ChatRequest {
    /// Create a new chat request with defaults
    pub fn new(message: String) -> Self {
        Self {
            message,
            session_id: None,
            think: None,
            model: None,
            temperature: Some(0.7),
            top_p: Some(0.9),
            max_tokens: Some(2048),
            stream: Some(true),
            system_prompt: None,
        }
    }

    /// Get the effective model
    pub fn get_model(&self) -> String {
        self.model.clone().unwrap_or_else(|| "GLM4_6".to_string())
    }

    /// Get the effective temperature
    pub fn get_temperature(&self) -> f32 {
        self.temperature.unwrap_or(0.7)
    }

    /// Get the effective top-p
    pub fn get_top_p(&self) -> f32 {
        self.top_p.unwrap_or(0.9)
    }

    /// Get the effective max tokens
    pub fn get_max_tokens(&self) -> u32 {
        self.max_tokens.unwrap_or(2048)
    }

    /// Check if streaming is enabled
    pub fn is_streaming(&self) -> bool {
        self.stream.unwrap_or(true)
    }

    /// Check if think mode is enabled
    pub fn is_think_mode(&self) -> bool {
        self.think.unwrap_or(false)
    }
}

/// Chat response payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    /// AI's response
    pub reply: String,

    /// Session ID for conversation continuity
    pub session_id: String,

    /// Response metadata
    pub metadata: ResponseMetadata,

    /// Usage statistics
    pub usage: Option<UsageStats>,
}

/// Response metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseMetadata {
    /// Model used for generation
    pub model: String,

    /// Whether think mode was enabled
    pub think_mode: bool,

    /// Generation parameters used
    pub parameters: GenerationParameters,

    /// Response timestamp
    pub timestamp: String,

    /// Processing time in milliseconds
    pub processing_time_ms: u64,
}

/// Generation parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationParameters {
    pub temperature: f32,
    pub top_p: f32,
    pub max_tokens: u32,
}

/// Usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageStats {
    /// Prompt tokens used
    pub prompt_tokens: u32,

    /// Completion tokens used
    pub completion_tokens: u32,

    /// Total tokens used
    pub total_tokens: u32,

    /// Estimated cost (optional)
    pub estimated_cost: Option<f64>,
}

/// Streaming response chunk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChunk {
    /// Content chunk
    pub content: String,

    /// Session ID
    pub session_id: String,

    /// Whether this is the final chunk
    pub done: bool,

    /// Chunk metadata
    pub metadata: Option<StreamMetadata>,

    /// Usage statistics (final chunk only)
    pub usage: Option<UsageStats>,
}

/// Streaming chunk metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamMetadata {
    /// Finish reason
    pub finish_reason: Option<String>,

    /// Model name
    pub model: Option<String>,

    /// Whether this chunk contains reasoning content
    pub has_reasoning: bool,
}

/// Chat completion request builder
pub struct ChatCompletionBuilder {
    model: GLM4_6,
    messages: Vec<TextMessage>,
    api_key: String,
    temperature: f32,
    top_p: f32,
    thinking: ThinkingType,
    stream: bool,
}

impl ChatCompletionBuilder {
    /// Create a new chat completion builder
    pub fn new(api_key: String) -> Self {
        Self {
            model: GLM4_6 {},
            messages: Vec::new(),
            api_key,
            temperature: 0.7,
            top_p: 0.9,
            thinking: ThinkingType::Disabled,
            stream: false,
        }
    }

    /// Set messages
    pub fn messages(mut self, messages: Vec<TextMessage>) -> Self {
        self.messages = messages;
        self
    }

    /// Set temperature
    pub fn temperature(mut self, temperature: f32) -> Self {
        self.temperature = temperature.clamp(0.0, 2.0);
        self
    }

    /// Set top-p
    pub fn top_p(mut self, top_p: f32) -> Self {
        self.top_p = top_p.clamp(0.0, 1.0);
        self
    }

    /// Enable think mode
    pub fn with_thinking(mut self, enabled: bool) -> Self {
        self.thinking = if enabled {
            ThinkingType::Enabled
        } else {
            ThinkingType::Disabled
        };
        self
    }

    /// Enable streaming
    pub fn with_streaming(mut self, enabled: bool) -> Self {
        self.stream = enabled;
        self
    }

    /// Build the chat completion client
    pub fn build(self) -> zai_rs::AppResult<zai_rs::model::ChatCompletion<GLM4_6, TextMessage>> {
        if self.messages.is_empty() {
            return Err(crate::client::error_handler::ClientError::InvalidRequest(
                "No messages provided".to_string(),
            ));
        }

        let mut client =
            zai_rs::model::ChatCompletion::new(self.model, self.messages[0].clone(), self.api_key)
                .with_temperature(self.temperature)
                .with_top_p(self.top_p)
                .with_thinking(self.thinking);

        // Add remaining messages
        for message in self.messages.into_iter().skip(1) {
            client = client.add_messages(message);
        }

        Ok(client)
    }
}

/// Helper functions for chat operations
pub mod chat_utils {
    use super::*;

    /// Create a system message
    pub fn system_message(content: impl Into<String>) -> TextMessage {
        TextMessage::system(content.into())
    }

    /// Create a user message
    pub fn user_message(content: impl Into<String>) -> TextMessage {
        TextMessage::user(content.into())
    }

    /// Create an assistant message
    pub fn assistant_message(content: impl Into<String>) -> TextMessage {
        TextMessage::assistant(content.into())
    }

    /// Extract text content from AI response
    pub fn extract_text_from_response(
        response: &zai_rs::model::ChatBaseResponse,
    ) -> Option<String> {
        response
            .choices()
            .and_then(|choices| choices.get(0))
            .and_then(|choice| choice.message().content())
            .and_then(|content| content.as_str())
            .map(|s| s.to_string())
    }

    /// Extract text from streaming chunk
    pub fn extract_text_from_chunk(chunk: &zai_rs::model::ChatStreamResponse) -> Option<String> {
        chunk.choices.get(0)?.delta.as_ref()?.content.clone()
    }

    /// Extract reasoning content from streaming chunk
    pub fn extract_reasoning_from_chunk(
        chunk: &zai_rs::model::ChatStreamResponse,
    ) -> Option<String> {
        chunk
            .choices
            .get(0)?
            .delta
            .as_ref()?
            .reasoning_content
            .clone()
    }

    /// Create a default system prompt
    pub fn default_system_prompt() -> String {
        r#"You are a helpful AI assistant. Please provide accurate, helpful, and friendly responses.

Guidelines:
- Be concise but comprehensive
- Use clear and simple language
- Provide examples when helpful
- Admit when you're unsure about something
- Keep responses relevant to the user's question"#
            .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chat_request_defaults() {
        let request = ChatRequest::new("Hello".to_string());

        assert_eq!(request.message, "Hello");
        assert_eq!(request.get_temperature(), 0.7);
        assert_eq!(request.get_top_p(), 0.9);
        assert_eq!(request.get_max_tokens(), 2048);
        assert!(request.is_streaming());
        assert!(!request.is_think_mode());
    }

    #[test]
    fn test_chat_completion_builder() {
        let builder = ChatCompletionBuilder::new("test-key".to_string())
            .temperature(0.8)
            .top_p(0.95)
            .with_thinking(true)
            .with_streaming(true);

        assert_eq!(builder.temperature, 0.8);
        assert_eq!(builder.top_p, 0.95);
        assert!(matches!(builder.thinking, ThinkingType::Enabled));
        assert!(builder.stream);
    }

    #[test]
    fn test_chat_utils() {
        let sys_msg = chat_utils::system_message("System message");
        assert_eq!(sys_msg.role, "system");
        assert_eq!(sys_msg.content, serde_json::json!("System message"));

        let user_msg = chat_utils::user_message("User message");
        assert_eq!(user_msg.role, "user");
        assert_eq!(user_msg.content, serde_json::json!("User message"));

        let assistant_msg = chat_utils::assistant_message("Assistant message");
        assert_eq!(assistant_msg.role, "assistant");
        assert_eq!(
            assistant_msg.content,
            serde_json::json!("Assistant message")
        );
    }
}
