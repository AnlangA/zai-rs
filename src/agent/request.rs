//! Shared, closed Agent v1 request values.

use serde::{Deserialize, Serialize};

/// Agent identifiers accepted by the frozen `/v1/agents` request schema.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentId {
    /// General-purpose translation agent.
    GeneralTranslation,
    /// Slide-deck generation agent.
    SlidesGlmAgent,
    /// AI drawing agent.
    AiDrawingAgent,
    /// Receipt-recognition agent.
    ReceiptRecognitionAgent,
    /// Clothing-recognition agent.
    ClothesRecognitionAgent,
    /// Education problem-solving agent.
    IntelligentEducationSolveAgent,
    /// Vidu template agent.
    ViduTemplateAgent,
}

impl AgentId {
    /// Return the exact identifier serialized on the wire.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::GeneralTranslation => "general_translation",
            Self::SlidesGlmAgent => "slides_glm_agent",
            Self::AiDrawingAgent => "ai_drawing_agent",
            Self::ReceiptRecognitionAgent => "receipt_recognition_agent",
            Self::ClothesRecognitionAgent => "clothes_recognition_agent",
            Self::IntelligentEducationSolveAgent => "intelligent_education_solve_agent",
            Self::ViduTemplateAgent => "vidu_template_agent",
        }
    }
}

impl std::fmt::Display for AgentId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Roles accepted in Agent v1 request messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AgentRole {
    /// System instruction.
    System,
    /// End-user input.
    User,
    /// Assistant-authored context.
    Assistant,
}

/// Request content-part discriminator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentRequestContentType {
    /// Text content.
    Text,
    /// Previously uploaded file identifier.
    FileId,
    /// Remote file URL.
    FileUrl,
    /// Remote image URL.
    ImageUrl,
}

/// One typed multimodal content part in an Agent v1 request.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentRequestContentPart {
    #[serde(rename = "type")]
    type_: AgentRequestContentType,
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    file_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    file_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    image_url: Option<String>,
}

impl std::fmt::Debug for AgentRequestContentPart {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AgentRequestContentPart")
            .field("type", &self.type_)
            .field("value", &"[REDACTED]")
            .finish()
    }
}

impl AgentRequestContentPart {
    fn with_value(type_: AgentRequestContentType, value: String) -> Self {
        let mut part = Self {
            type_,
            text: None,
            file_id: None,
            file_url: None,
            image_url: None,
        };
        match type_ {
            AgentRequestContentType::Text => part.text = Some(value),
            AgentRequestContentType::FileId => part.file_id = Some(value),
            AgentRequestContentType::FileUrl => part.file_url = Some(value),
            AgentRequestContentType::ImageUrl => part.image_url = Some(value),
        }
        part
    }

    /// Construct a text part.
    pub fn text(value: impl Into<String>) -> Self {
        Self::with_value(AgentRequestContentType::Text, value.into())
    }

    /// Construct an uploaded-file identifier part.
    pub fn file_id(value: impl Into<String>) -> Self {
        Self::with_value(AgentRequestContentType::FileId, value.into())
    }

    /// Construct a remote-file URL part.
    pub fn file_url(value: impl Into<String>) -> Self {
        Self::with_value(AgentRequestContentType::FileUrl, value.into())
    }

    /// Construct a remote-image URL part.
    pub fn image_url(value: impl Into<String>) -> Self {
        Self::with_value(AgentRequestContentType::ImageUrl, value.into())
    }

    /// Return the content-part discriminator.
    pub const fn type_(&self) -> AgentRequestContentType {
        self.type_
    }
}

/// Agent request content in one of the three frozen wire forms.
#[derive(Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AgentContent {
    /// Plain text content.
    Text(String),
    /// One multimodal content part.
    Part(AgentRequestContentPart),
    /// Multiple multimodal content parts.
    Parts(Vec<AgentRequestContentPart>),
}

impl std::fmt::Debug for AgentContent {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Text(_) => formatter.write_str("AgentContent::Text([REDACTED])"),
            Self::Part(part) => formatter
                .debug_tuple("AgentContent::Part")
                .field(part)
                .finish(),
            Self::Parts(parts) => formatter
                .debug_struct("AgentContent::Parts")
                .field("count", &parts.len())
                .finish(),
        }
    }
}

impl From<String> for AgentContent {
    fn from(value: String) -> Self {
        Self::Text(value)
    }
}

impl From<&str> for AgentContent {
    fn from(value: &str) -> Self {
        Self::Text(value.to_owned())
    }
}

impl From<AgentRequestContentPart> for AgentContent {
    fn from(value: AgentRequestContentPart) -> Self {
        Self::Part(value)
    }
}

impl From<Vec<AgentRequestContentPart>> for AgentContent {
    fn from(value: Vec<AgentRequestContentPart>) -> Self {
        Self::Parts(value)
    }
}

/// One message in an Agent v1 invocation request.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentMessage {
    role: AgentRole,
    content: AgentContent,
}

impl std::fmt::Debug for AgentMessage {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AgentMessage")
            .field("role", &self.role)
            .field("content", &"[REDACTED]")
            .finish()
    }
}

impl AgentMessage {
    /// Construct a message with an explicit typed role.
    pub fn new(role: AgentRole, content: impl Into<AgentContent>) -> Self {
        Self {
            role,
            content: content.into(),
        }
    }

    /// Construct a user message.
    pub fn user(content: impl Into<AgentContent>) -> Self {
        Self::new(AgentRole::User, content)
    }

    /// Construct a system message.
    pub fn system(content: impl Into<AgentContent>) -> Self {
        Self::new(AgentRole::System, content)
    }

    /// Construct an assistant message.
    pub fn assistant(content: impl Into<AgentContent>) -> Self {
        Self::new(AgentRole::Assistant, content)
    }

    /// Return the message role.
    pub const fn role(&self) -> AgentRole {
        self.role
    }

    /// Borrow the typed message content.
    pub const fn content(&self) -> &AgentContent {
        &self.content
    }
}
