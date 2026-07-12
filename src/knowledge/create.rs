use serde::{Deserialize, Deserializer, Serialize, Serializer};
use validator::Validate;

use super::types::KnowledgeResponse;
use crate::ZaiResult;
use crate::client::ZaiClient;

/// Embedding model identifier encoded as the service's integer value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmbeddingId {
    /// 3: Embedding-2
    Embedding2,
    /// 11: Embedding-3
    Embedding3New,
    /// 12: Embedding-3-pro
    Embedding3Pro,
}

impl EmbeddingId {
    /// Return the upstream integer id for this embedding model.
    pub fn as_i64(&self) -> i64 {
        match self {
            EmbeddingId::Embedding2 => 3,
            EmbeddingId::Embedding3New => 11,
            EmbeddingId::Embedding3Pro => 12,
        }
    }

    /// Return the canonical model name paired with this identifier.
    pub fn as_model_name(&self) -> &'static str {
        match self {
            EmbeddingId::Embedding2 => "Embedding-2",
            EmbeddingId::Embedding3New => "Embedding-3",
            EmbeddingId::Embedding3Pro => "Embedding-3-pro",
        }
    }
}

impl Serialize for EmbeddingId {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_i64(self.as_i64())
    }
}

impl<'de> Deserialize<'de> for EmbeddingId {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let v = i64::deserialize(deserializer)?;
        match v {
            3 => Ok(EmbeddingId::Embedding2),
            11 => Ok(EmbeddingId::Embedding3New),
            12 => Ok(EmbeddingId::Embedding3Pro),
            other => Err(serde::de::Error::custom(format!(
                "unsupported embedding_id: {other} (expected 3, 11 or 12)"
            ))),
        }
    }
}

/// Background color enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BackgroundColor {
    /// Blue background.
    Blue,
    /// Red background.
    Red,
    /// Orange background.
    Orange,
    /// Purple background.
    Purple,
    /// Sky-blue background.
    Sky,
    /// Green background.
    Green,
    /// Yellow background.
    Yellow,
}

/// Knowledge icon enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum KnowledgeIcon {
    /// Question-mark icon.
    Question,
    /// Book icon.
    Book,
    /// Seal icon.
    Seal,
    /// Wrench icon.
    Wrench,
    /// Tag icon.
    Tag,
    /// Horn icon.
    Horn,
    /// House icon.
    House,
}

/// Request body for creating a knowledge base
#[derive(Clone, Serialize, Deserialize, Validate)]
pub struct KnowledgeCreateBody {
    /// Embedding model ID (3, 11, or 12).
    pub embedding_id: EmbeddingId,
    /// Knowledge base name
    #[validate(length(min = 1))]
    pub name: String,
    /// Knowledge base description (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Background color (optional; default blue on server)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background: Option<BackgroundColor>,
    /// Icon name (optional; default question on server)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<KnowledgeIcon>,
    /// Embedding model name (optional). When given alongside `embedding_id`,
    /// the service requires the two to be consistent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding_model: Option<String>,
    /// Contextual retrieval flag (`0` or `1`).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(range(max = 1))]
    pub contextual: Option<u8>,
}

impl std::fmt::Debug for KnowledgeCreateBody {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("KnowledgeCreateBody")
            .field("embedding_id", &self.embedding_id)
            .field("name", &"[REDACTED]")
            .field("description_configured", &self.description.is_some())
            .field("background", &self.background)
            .field("icon", &self.icon)
            .field(
                "embedding_model_configured",
                &self.embedding_model.is_some(),
            )
            .field("contextual", &self.contextual)
            .finish()
    }
}

impl KnowledgeCreateBody {
    fn validate_embedding_pair(&self) -> ZaiResult<()> {
        if let Some(model) = self.embedding_model.as_deref()
            && model != self.embedding_id.as_model_name()
        {
            return Err(crate::ZaiError::ApiError {
                code: crate::client::error::codes::SDK_VALIDATION,
                message: format!(
                    "embedding_id {} requires embedding_model '{}'",
                    self.embedding_id.as_i64(),
                    self.embedding_id.as_model_name()
                ),
            });
        }
        Ok(())
    }
}

/// Create knowledge request (POST /llm-application/open/knowledge)
///
/// Credentials and transport live on the [`ZaiClient`], passed to
/// [`send_via`](Self::send_via).
pub struct KnowledgeCreateRequest {
    body: KnowledgeCreateBody,
}

impl KnowledgeCreateRequest {
    /// Build a create request with required fields
    pub fn new(embedding_id: EmbeddingId, name: impl Into<String>) -> Self {
        let body = KnowledgeCreateBody {
            embedding_id,
            name: name.into(),
            description: None,
            background: None,
            icon: None,
            embedding_model: None,
            contextual: None,
        };
        Self { body }
    }

    /// Set the knowledge-base description.
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.body.description = Some(desc.into());
        self
    }
    /// Set the background color.
    pub fn with_background(mut self, bg: BackgroundColor) -> Self {
        self.body.background = Some(bg);
        self
    }
    /// Set the icon.
    pub fn with_icon(mut self, icon: KnowledgeIcon) -> Self {
        self.body.icon = Some(icon);
        self
    }
    /// Set the embedding model name. It must match the configured
    /// [`EmbeddingId`]; [`Self::send_via`] validates the pair locally.
    pub fn with_embedding_model(mut self, model: impl Into<String>) -> Self {
        self.body.embedding_model = Some(model.into());
        self
    }
    /// Set the contextual-retrieval flag (`0` or `1`).
    ///
    /// [`Self::send_via`] rejects values other than `0` and `1`.
    pub fn with_contextual(mut self, contextual: u8) -> Self {
        self.body.contextual = Some(contextual);
        self
    }

    /// Validate all documented request constraints without performing I/O.
    pub fn validate(&self) -> ZaiResult<()> {
        self.body.validate()?;
        if self.body.name.trim().is_empty() {
            return Err(crate::client::validation::invalid(
                "knowledge name must not be blank",
            ));
        }
        self.body.validate_embedding_pair()
    }

    /// Validate and send via a [`ZaiClient`], returning the typed response.
    pub async fn send_via(&self, client: &ZaiClient) -> ZaiResult<KnowledgeCreateResponse> {
        self.validate()?;
        let route = crate::client::routes::KNOWLEDGE_CREATE;
        let url = client.endpoints().resolve_route(route, &[])?;
        client
            .send_json::<_, KnowledgeCreateResponse>(route.method(), url, &self.body)
            .await
    }
}

/// Inner data of [`KnowledgeCreateResponse`] — the newly created id.
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct KnowledgeCreateData {
    /// Newly created id
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
}

/// Response of knowledge creation.
pub type KnowledgeCreateResponse = KnowledgeResponse<KnowledgeCreateData>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_blank_names_and_mismatched_embedding_models() {
        assert!(
            KnowledgeCreateRequest::new(EmbeddingId::Embedding3New, " \t")
                .validate()
                .is_err()
        );
        assert!(
            KnowledgeCreateRequest::new(EmbeddingId::Embedding3New, "docs")
                .with_embedding_model("Embedding-2")
                .validate()
                .is_err()
        );
        assert!(
            KnowledgeCreateRequest::new(EmbeddingId::Embedding3New, "docs")
                .with_embedding_model("Embedding-3")
                .validate()
                .is_ok()
        );
    }

    #[test]
    fn request_body_debug_redacts_names_and_descriptions() {
        let body = KnowledgeCreateBody {
            embedding_id: EmbeddingId::Embedding3New,
            name: "private-name".to_owned(),
            description: Some("private-description".to_owned()),
            background: Some(BackgroundColor::Blue),
            icon: Some(KnowledgeIcon::Book),
            embedding_model: Some("private-model-string".to_owned()),
            contextual: Some(1),
        };
        let debug = format!("{body:?}");
        for secret in [
            "private-name",
            "private-description",
            "private-model-string",
        ] {
            assert!(!debug.contains(secret));
        }
    }
}
