use serde::{Deserialize, Serialize};
use validator::{Validate, ValidationError};
use zai_rs::model::{
    ChatCompletion, TextMessage, ThinkingType,
    chat_base_response::{ChatCompletionResponse, Usage},
    chat_models::GLM4_6,
};

use crate::server::error::{AppError, AppResult};

const MAX_SESSION_ID_BYTES: usize = 128;

#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct ChatRequest {
    #[validate(length(min = 1, max = 10_000), custom(function = "validate_nonblank"))]
    message: String,
    #[validate(custom(function = "validate_session_id"))]
    session_id: Option<String>,
    think: Option<bool>,
    #[validate(range(min = 0.0, max = 1.0))]
    temperature: Option<f64>,
    #[validate(range(min = 1, max = 8192))]
    max_tokens: Option<u32>,
}

fn validate_nonblank(value: &str) -> Result<(), ValidationError> {
    if value.trim().is_empty() {
        return Err(ValidationError::new("blank"));
    }
    Ok(())
}

fn validate_session_id(value: &str) -> Result<(), ValidationError> {
    if is_valid_session_id(value) {
        return Ok(());
    }
    Err(ValidationError::new("unsafe_characters"))
}

fn is_valid_session_id(value: &str) -> bool {
    (1..=MAX_SESSION_ID_BYTES).contains(&value.len())
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

/// Validate session identifiers supplied in route paths. Request-body IDs use
/// the same predicate through `validator` above.
pub fn ensure_valid_session_id(value: &str) -> AppResult<()> {
    if is_valid_session_id(value) {
        Ok(())
    } else {
        Err(AppError::InvalidRequest(
            "session_id must contain 1-128 ASCII letters, digits, '-' or '_'".to_owned(),
        ))
    }
}

impl ChatRequest {
    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn session_id(&self) -> Option<&str> {
        self.session_id.as_deref()
    }

    pub fn thinking_enabled(&self) -> bool {
        self.think.unwrap_or(false)
    }

    pub fn temperature(&self) -> f64 {
        self.temperature.unwrap_or(0.7)
    }

    pub fn max_tokens(&self) -> u32 {
        self.max_tokens.unwrap_or(2048)
    }
}

#[derive(Debug, Serialize)]
pub struct ChatResponse {
    pub reply: String,
    pub session_id: String,
    pub metadata: ResponseMetadata,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<UsageStats>,
}

#[derive(Debug, Serialize)]
pub struct ResponseMetadata {
    pub model: String,
    pub think_mode: bool,
    pub temperature: f64,
    pub max_tokens: u32,
    pub timestamp: String,
    pub processing_time_ms: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct UsageStats {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

impl From<&Usage> for UsageStats {
    fn from(usage: &Usage) -> Self {
        Self {
            prompt_tokens: usage.prompt_tokens.unwrap_or_default(),
            completion_tokens: usage.completion_tokens.unwrap_or_default(),
            total_tokens: usage.total_tokens.unwrap_or_default(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct StreamChunk {
    pub content: String,
    pub session_id: String,
    pub done: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<StreamMetadata>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<UsageStats>,
}

#[derive(Debug, Serialize)]
pub struct StreamMetadata {
    pub finish_reason: Option<String>,
    pub model: Option<String>,
    pub has_reasoning: bool,
}

pub type ChatModelRequest = ChatCompletion<GLM4_6, TextMessage>;

pub fn build_completion(
    messages: &[TextMessage],
    request: &ChatRequest,
) -> AppResult<ChatModelRequest> {
    let (first, remaining) = messages
        .split_first()
        .ok_or_else(|| AppError::InvalidRequest("conversation is empty".to_owned()))?;
    let thinking = if request.thinking_enabled() {
        ThinkingType::enabled()
    } else {
        ThinkingType::disabled()
    };
    let mut completion = ChatCompletion::new(GLM4_6 {}, first.clone())
        .with_temperature(request.temperature())
        .with_max_tokens(request.max_tokens())
        .with_thinking(thinking);
    for message in remaining {
        completion = completion.add_message(message.clone());
    }
    Ok(completion)
}

pub fn response_text(response: &ChatCompletionResponse) -> Option<String> {
    response
        .choices()
        .and_then(|choices| choices.first())
        .and_then(|choice| choice.message())
        .and_then(|message| message.content_str())
        .filter(|content| !content.trim().is_empty())
        .map(str::to_owned)
}

pub fn message_role(message: &TextMessage) -> &'static str {
    match message {
        TextMessage::User { .. } => "user",
        TextMessage::Assistant { .. } => "assistant",
        TextMessage::System { .. } => "system",
        TextMessage::Tool { .. } => "tool",
    }
}

pub fn message_text(message: &TextMessage) -> String {
    match message {
        TextMessage::User { content }
        | TextMessage::System { content }
        | TextMessage::Tool { content, .. } => content.clone(),
        TextMessage::Assistant { content, .. } => content.clone().unwrap_or_default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(message: &str) -> ChatRequest {
        ChatRequest {
            message: message.to_owned(),
            session_id: None,
            think: None,
            temperature: None,
            max_tokens: None,
        }
    }

    #[test]
    fn rejects_values_outside_the_core_chat_contract() {
        assert!(request(" \n ").validate().is_err());

        let mut invalid_temperature = request("hello");
        invalid_temperature.temperature = Some(1.01);
        assert!(invalid_temperature.validate().is_err());

        let mut empty_session = request("hello");
        empty_session.session_id = Some(String::new());
        assert!(empty_session.validate().is_err());

        let mut unsafe_session = request("hello");
        unsafe_session.session_id = Some("session/../other".to_owned());
        assert!(unsafe_session.validate().is_err());

        assert!(ensure_valid_session_id(&"x".repeat(129)).is_err());
    }

    #[test]
    fn unknown_request_fields_are_rejected() {
        let error = serde_json::from_str::<ChatRequest>(r#"{"message":"hello","temprature":0.5}"#)
            .unwrap_err();
        assert!(error.to_string().contains("unknown field"));
    }
}
