use std::{collections::HashMap, sync::Arc};

use serde::{Deserialize, Serialize};
use validator::Validate;

use super::create::{BackgroundColor, EmbeddingId, KnowledgeIcon};
use crate::client::ZaiClient;
use crate::client::http::{HttpClientConfig, parse_typed_response, send_json_request};

/// Update body for editing a knowledge base
#[derive(Debug, Clone, Default, Serialize, Deserialize, Validate)]
pub struct UpdateKnowledgeBody {
    /// Embedding model id (3 or 11)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding_id: Option<EmbeddingId>,
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
    pub callback_url: Option<String>,
    /// Callback headers as key-value
    #[serde(skip_serializing_if = "Option::is_none")]
    pub callback_header: Option<HashMap<String, String>>,
}

impl UpdateKnowledgeBody {
    /// Returns true if no fields are set
    fn is_empty(&self) -> bool {
        self.embedding_id.is_none()
            && self.name.is_none()
            && self.description.is_none()
            && self.background.is_none()
            && self.icon.is_none()
            && self.callback_url.is_none()
            && self.callback_header.is_none()
    }
}

/// Knowledge update request (PUT /llm-application/open/knowledge/{id})
///
/// Credentials and transport live on the [`ZaiClient`], passed to
/// [`send_via`](Self::send_via).
pub struct KnowledgeUpdateRequest {
    id: String,
    body: UpdateKnowledgeBody,
}

impl KnowledgeUpdateRequest {
    /// Build update request targeting a specific id with empty body.
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            body: UpdateKnowledgeBody::default(),
        }
    }

    /// Setters to update individual fields
    /// Set the embedding model id.
    pub fn with_embedding_id(mut self, id: EmbeddingId) -> Self {
        self.body.embedding_id = Some(id);
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
    pub fn with_callback_header(mut self, headers: HashMap<String, String>) -> Self {
        self.body.callback_header = Some(headers);
        self
    }

    /// Send the update request via a [`ZaiClient`] and parse the typed response.
    pub async fn send_via(&self, client: &ZaiClient) -> crate::ZaiResult<KnowledgeUpdateResponse> {
        if self.body.is_empty() {
            return Err(crate::client::error::ZaiError::ApiError {
                code: crate::client::error::codes::SDK_VALIDATION,
                message: "update body is empty; set at least one field".to_string(),
            });
        }
        self.body.validate()?;
        let url = client.endpoints().resolve(
            crate::client::ApiFamily::LlmApplication,
            &["knowledge", &self.id],
        )?;
        let config = transport_config_from_client(client);
        let resp = send_json_request(
            reqwest::Method::PUT,
            url,
            client.secret().expose(),
            &self.body,
            Arc::new(config),
        )
        .await?;
        parse_typed_response::<KnowledgeUpdateResponse>(resp).await
    }
}

/// Update response envelope without data
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct KnowledgeUpdateResponse {
    /// Business status code.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<i64>,
    /// Human-readable message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// Server timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<u64>,
}

fn transport_config_from_client(client: &ZaiClient) -> HttpClientConfig {
    let t = client.transport();
    HttpClientConfig {
        timeout: std::time::Duration::from_secs(t.request_timeout.as_secs()),
        max_retries: u32::from(t.max_attempts).saturating_sub(1),
        enable_compression: t.enable_compression,
        retry_delay: crate::client::http::RetryDelay::Exponential {
            base: std::time::Duration::from_millis(500),
            max: std::time::Duration::from_secs(5),
        },
        enable_logging: false,
        mask_sensitive_data: true,
    }
}
