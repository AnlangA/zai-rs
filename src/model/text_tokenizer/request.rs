use serde::{Deserialize, Serialize};

/// Tokenizer-capable models
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TokenizerModel {
    /// glm-4.6v (current service default).
    #[serde(rename = "glm-4.6v")]
    #[default]
    Glm46V,
    /// glm-4.6.
    #[serde(rename = "glm-4.6")]
    Glm46,
    /// glm-4.5.
    #[serde(rename = "glm-4.5")]
    Glm45,
    /// glm-4.5-air.
    #[serde(rename = "glm-4.5-air")]
    Glm45Air,
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
#[derive(Clone, Serialize, Deserialize)]
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
}

impl std::fmt::Debug for TokenizerMessage {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (role, configured) = match self {
            Self::User { .. } => ("user", true),
            Self::System { .. } => ("system", true),
            Self::Assistant { content } => ("assistant", content.is_some()),
        };
        formatter
            .debug_struct("TokenizerMessage")
            .field("role", &role)
            .field("content_configured", &configured)
            .finish()
    }
}

/// Request body for tokenizer
#[derive(Clone, Serialize, Deserialize)]
pub struct TokenizerBody {
    /// Model used for token counting (defaults to `glm-4.6v`).
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

impl std::fmt::Debug for TokenizerBody {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("TokenizerBody")
            .field("model", &self.model)
            .field("messages_len", &self.messages.len())
            .field(
                "request_id",
                &self.request_id.as_ref().map(|_| "[REDACTED]"),
            )
            .field("user_id", &self.user_id.as_ref().map(|_| "[REDACTED]"))
            .finish()
    }
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

    /// Validate constraints shared by direct body users and the request client.
    pub fn validate(&self) -> crate::ZaiResult<()> {
        if self.messages.is_empty() {
            return Err(crate::ZaiError::ApiError {
                code: crate::client::error::codes::SDK_VALIDATION,
                message: "messages must not be empty".to_owned(),
            });
        }
        let has_blank_content = self.messages.iter().any(|message| match message {
            TokenizerMessage::User { content } | TokenizerMessage::System { content } => {
                content.trim().is_empty()
            },
            TokenizerMessage::Assistant { content } => content
                .as_deref()
                .is_some_and(|content| content.trim().is_empty()),
        });
        if has_blank_content {
            return Err(crate::ZaiError::ApiError {
                code: crate::client::error::codes::SDK_VALIDATION,
                message: "message content must not be blank when present".to_owned(),
            });
        }
        if let Some(request_id) = self.request_id.as_deref()
            && !(6..=64).contains(&request_id.chars().count())
        {
            return Err(crate::ZaiError::ApiError {
                code: crate::client::error::codes::SDK_VALIDATION,
                message: "request_id must contain between 6 and 64 characters".to_owned(),
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_default_and_model_ids_match_the_contract() {
        assert_eq!(
            serde_json::to_value(TokenizerModel::default()).unwrap(),
            "glm-4.6v"
        );
        assert_eq!(
            serde_json::to_value(TokenizerModel::Glm46).unwrap(),
            "glm-4.6"
        );
        for (model, expected) in [
            (TokenizerModel::Glm46V, "glm-4.6v"),
            (TokenizerModel::Glm46, "glm-4.6"),
            (TokenizerModel::Glm45, "glm-4.5"),
            (TokenizerModel::Glm45Air, "glm-4.5-air"),
            (TokenizerModel::Glm40520, "glm-4-0520"),
            (TokenizerModel::Glm4Long, "glm-4-long"),
            (TokenizerModel::Glm4Air, "glm-4-air"),
            (TokenizerModel::Glm4Flash, "glm-4-flash"),
        ] {
            assert_eq!(serde_json::to_value(model).unwrap(), expected);
        }
    }

    #[test]
    fn validation_rejects_blank_content_and_short_request_ids() {
        let body = TokenizerBody::new(
            TokenizerModel::default(),
            vec![TokenizerMessage::User {
                content: " ".into(),
            }],
        );
        assert!(body.validate().is_err());

        let body = TokenizerBody::new(
            TokenizerModel::default(),
            vec![TokenizerMessage::User {
                content: "hello".into(),
            }],
        )
        .with_request_id("short");
        assert!(body.validate().is_err());
    }

    #[test]
    fn debug_redacts_message_content_and_identifiers() {
        let body = TokenizerBody::new(
            TokenizerModel::default(),
            vec![TokenizerMessage::User {
                content: "private tokenizer input".to_owned(),
            }],
        )
        .with_request_id("private-request")
        .with_user_id("private-user");
        let debug = format!("{body:?}");
        for secret in ["private tokenizer input", "private-request", "private-user"] {
            assert!(!debug.contains(secret));
        }
        assert!(debug.contains("messages_len: 1"));
    }
}
