use serde::{Deserialize, Serialize};

/// Tokenizer-capable models
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[derive(Default)]
pub enum TokenizerModel {
    /// glm-4-plus (default).
    #[serde(rename = "glm-4-plus")]
    #[default]
    Glm4Plus,
    /// glm-4-0520.
    #[serde(rename = "glm-4-0520")]
    Glm40520,
    /// glm-4-long.
    #[serde(rename = "glm-4-long")]
    Glm4Long,
    /// glm-4-air.
    #[serde(rename = "glm-4-air")]
    Glm4Air,
    /// glm-4-flash.
    #[serde(rename = "glm-4-flash")]
    Glm4Flash,
}

/// One message item for tokenizer input
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "role", rename_all = "lowercase")]
pub enum TokenizerMessage {
    /// User message.
    User {
        /// Message text.
        content: String,
    },
    /// System instruction.
    System {
        /// System-instruction text.
        content: String,
    },
    /// Assistant message with optional content.
    Assistant {
        /// Assistant reply text, if any.
        #[serde(skip_serializing_if = "Option::is_none")]
        content: Option<String>,
    },
    /// Tool-result message.
    Tool {
        /// Tool-result text.
        content: String,
    },
}

/// Request body for tokenizer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenizerBody {
    /// Model used for token counting (defaults to `glm-4-plus`).
    pub model: TokenizerModel,
    /// Conversation messages; at least one is required.
    pub messages: Vec<TokenizerMessage>,
    /// Client-provided request identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    /// End-user identifier used for abuse monitoring.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
}

impl TokenizerBody {
    /// Create a new tokenizer body from a model and a message list.
    pub fn new(model: TokenizerModel, messages: Vec<TokenizerMessage>) -> Self {
        Self {
            model,
            messages,
            request_id: None,
            user_id: None,
        }
    }
    /// Set the client-side request id.
    pub fn with_request_id(mut self, v: impl Into<String>) -> Self {
        self.request_id = Some(v.into());
        self
    }
    /// Set the end-user id.
    pub fn with_user_id(mut self, v: impl Into<String>) -> Self {
        self.user_id = Some(v.into());
        self
    }
}
