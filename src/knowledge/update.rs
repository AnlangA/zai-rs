use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use validator::Validate;

use super::create::{BackgroundColor, EmbeddingId, KnowledgeIcon};
use super::types::KnowledgeOperationResponse;
use crate::client::ZaiClient;

/// Update body for editing a knowledge base
#[derive(Clone, Default, Serialize, Deserialize, Validate)]
pub struct KnowledgeUpdateBody {
    /// Embedding model ID (3, 11, or 12).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding_id: Option<EmbeddingId>,
    /// Embedding model name. When `embedding_id` is also set, both values must
    /// identify the same model.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding_model: Option<String>,
    /// Contextual retrieval flag (`0` or `1`).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(range(max = 1))]
    pub contextual: Option<u8>,
    /// Knowledge base name
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(length(min = 1))]
    pub name: Option<String>,
    /// Knowledge base description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Background color
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background: Option<BackgroundColor>,
    /// Icon name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<KnowledgeIcon>,
    /// Callback URL when rebuilding is required
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(url)]
    pub callback_url: Option<String>,
    /// Callback headers as key-value
    #[serde(skip_serializing_if = "Option::is_none")]
    pub callback_header: Option<BTreeMap<String, String>>,
}

impl std::fmt::Debug for KnowledgeUpdateBody {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("KnowledgeUpdateBody")
            .field("embedding_id", &self.embedding_id)
            .field(
                "embedding_model_configured",
                &self.embedding_model.is_some(),
            )
            .field("contextual", &self.contextual)
            .field("name_configured", &self.name.is_some())
            .field("description_configured", &self.description.is_some())
            .field("background", &self.background)
            .field("icon", &self.icon)
            .field("callback_url_configured", &self.callback_url.is_some())
            .field(
                "callback_header_entries",
                &self.callback_header.as_ref().map(BTreeMap::len),
            )
            .finish()
    }
}

impl KnowledgeUpdateBody {
    fn validate_embedding_pair(&self) -> crate::ZaiResult<()> {
        let Some(model) = self.embedding_model.as_deref() else {
            return Ok(());
        };
        if !matches!(model, "Embedding-2" | "Embedding-3" | "Embedding-3-pro") {
            return Err(crate::client::validation::invalid(
                "embedding_model must be one of: Embedding-2, Embedding-3, Embedding-3-pro",
            ));
        }
        if let Some(id) = self.embedding_id
            && model != id.as_model_name()
        {
            return Err(crate::client::validation::invalid(format!(
                "embedding_id {} requires embedding_model '{}'",
                id.as_i64(),
                id.as_model_name()
            )));
        }
        Ok(())
    }
}

/// Knowledge update request (PUT /llm-application/open/knowledge/{id})
///
/// Credentials and transport live on the [`ZaiClient`], passed to
/// [`send_via`](Self::send_via).
pub struct KnowledgeUpdateRequest {
    id: String,
    body: KnowledgeUpdateBody,
}

impl KnowledgeUpdateRequest {
    /// Build update request targeting a specific id with empty body.
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            body: KnowledgeUpdateBody::default(),
        }
    }

    /// Set the embedding model id.
    pub fn with_embedding_id(mut self, id: EmbeddingId) -> Self {
        self.body.embedding_id = Some(id);
        self
    }
    /// Set the embedding model name.
    pub fn with_embedding_model(mut self, model: impl Into<String>) -> Self {
        self.body.embedding_model = Some(model.into());
        self
    }
    /// Enable (`1`) or disable (`0`) contextual retrieval.
    pub fn with_contextual(mut self, contextual: u8) -> Self {
        self.body.contextual = Some(contextual);
        self
    }
    /// Set the knowledge-base name.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.body.name = Some(name.into());
        self
    }
    /// Set the description.
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
    /// Set the callback URL (notified when rebuilding completes).
    pub fn with_callback_url(mut self, url: impl Into<String>) -> Self {
        self.body.callback_url = Some(url.into());
        self
    }
    /// Set the callback headers.
    pub fn with_callback_header(mut self, headers: BTreeMap<String, String>) -> Self {
        self.body.callback_header = Some(headers);
        self
    }

    /// Validate the target identifier and update body without performing I/O.
    pub fn validate(&self) -> crate::ZaiResult<()> {
        crate::client::validation::require_non_blank(&self.id, "knowledge_id")?;
        self.body.validate()?;
        if self
            .body
            .name
            .as_deref()
            .is_some_and(|name| name.trim().is_empty())
        {
            return Err(crate::client::validation::invalid(
                "knowledge name must not be blank",
            ));
        }
        self.body.validate_embedding_pair()
    }

    /// Send the update request via a [`ZaiClient`] and parse the typed response.
    pub async fn send_via(&self, client: &ZaiClient) -> crate::ZaiResult<KnowledgeUpdateResponse> {
        self.validate()?;
        let route = crate::client::routes::KNOWLEDGE_UPDATE;
        let url = client.endpoints().resolve_route(route, &[&self.id])?;
        client
            .send_json::<_, KnowledgeUpdateResponse>(route.method(), url, &self.body)
            .await
    }
}

/// Update response envelope without a data payload.
pub type KnowledgeUpdateResponse = KnowledgeOperationResponse;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_blank_target_and_blank_name_without_inventing_required_fields() {
        assert!(
            KnowledgeUpdateRequest::new(" ")
                .with_description("value")
                .validate()
                .is_err()
        );
        assert!(KnowledgeUpdateRequest::new("kb-1").validate().is_ok());
        assert!(
            KnowledgeUpdateRequest::new("kb-1")
                .with_name(" \t")
                .validate()
                .is_err()
        );
        assert!(
            KnowledgeUpdateRequest::new("kb-1")
                .with_name("docs")
                .validate()
                .is_ok()
        );
        assert_eq!(
            serde_json::to_value(&KnowledgeUpdateRequest::new("kb-1").body).unwrap(),
            serde_json::json!({})
        );
    }

    #[test]
    fn request_body_debug_redacts_text_callbacks_and_headers() {
        let body = KnowledgeUpdateBody {
            embedding_model: Some("private-model".to_owned()),
            name: Some("private-name".to_owned()),
            description: Some("private-description".to_owned()),
            callback_url: Some("https://private.example/callback".to_owned()),
            callback_header: Some(BTreeMap::from([(
                "Authorization".to_owned(),
                "private-token".to_owned(),
            )])),
            ..KnowledgeUpdateBody::default()
        };
        let debug = format!("{body:?}");
        for secret in [
            "private-model",
            "private-name",
            "private-description",
            "private.example",
            "Authorization",
            "private-token",
        ] {
            assert!(!debug.contains(secret));
        }
        assert!(debug.contains("callback_header_entries: Some(1)"));
    }
}
