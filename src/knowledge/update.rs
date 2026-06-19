use std::{collections::HashMap, sync::Arc};

use serde::{Deserialize, Serialize};
use validator::Validate;

use super::create::{BackgroundColor, EmbeddingId, KnowledgeIcon};
use crate::client::{
    endpoints::{ApiBase, EndpointConfig, join_url, paths},
    http::{HttpClient, HttpClientConfig, parse_typed_response},
};

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
pub struct KnowledgeUpdateRequest {
    /// Bearer API key
    pub key: String,
    url: String,
    endpoint_config: EndpointConfig,
    api_base: ApiBase,
    id: String,
    http_config: Arc<HttpClientConfig>,
    body: UpdateKnowledgeBody,
}

impl KnowledgeUpdateRequest {
    /// Build update request targeting a specific id with empty body
    pub fn new(key: String, id: impl AsRef<str>) -> Self {
        let id = id.as_ref().to_string();
        let endpoint_config = EndpointConfig::default();
        let api_base = ApiBase::LlmApplication;
        let url = endpoint_config.url(&api_base, &join_url(paths::KNOWLEDGE, &id));
        Self {
            key,
            url,
            endpoint_config,
            api_base,
            id,
            http_config: Arc::new(HttpClientConfig::default()),
            body: UpdateKnowledgeBody::default(),
        }
    }

    fn rebuild_url(&mut self) {
        self.url = self
            .endpoint_config
            .url(&self.api_base, &join_url(paths::KNOWLEDGE, &self.id));
    }

    /// Override the base URL (uses [`ApiBase::Custom`]).
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.api_base = ApiBase::Custom(base_url.into());
        self.rebuild_url();
        self
    }

    /// Replace the full [`EndpointConfig`] used to resolve URLs.
    pub fn with_endpoint_config(mut self, endpoint_config: EndpointConfig) -> Self {
        self.endpoint_config = endpoint_config;
        self.rebuild_url();
        self
    }

    /// Replace the HTTP client configuration (timeouts, retries, …).
    pub fn with_http_config(mut self, config: HttpClientConfig) -> Self {
        self.http_config = Arc::new(config);
        self
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

    /// Send the update request and parse the typed response.
    pub async fn send(&self) -> crate::ZaiResult<KnowledgeUpdateResponse> {
        if self.body.is_empty() {
            return Err(crate::client::error::ZaiError::ApiError {
                code: 1200,
                message: "update body is empty; set at least one field".to_string(),
            });
        }

        self.body.validate()?;
        let resp = self.put().await?;
        parse_typed_response::<KnowledgeUpdateResponse>(resp).await
    }

    /// Issue the underlying PUT request.
    pub fn put(
        &self,
    ) -> impl std::future::Future<Output = crate::ZaiResult<reqwest::Response>> + Send {
        HttpClient::put(self)
    }
}

impl HttpClient for KnowledgeUpdateRequest {
    type Body = UpdateKnowledgeBody;
    type ApiUrl = String;
    type ApiKey = String;

    /// Resolved target URL for the request.
    fn api_url(&self) -> &Self::ApiUrl {
        &self.url
    }
    /// API key used for `Authorization: Bearer …`.
    fn api_key(&self) -> &Self::ApiKey {
        &self.key
    }
    /// Serialized request body.
    fn body(&self) -> &Self::Body {
        &self.body
    }

    /// HTTP client configuration (timeouts, retries, …).
    fn http_config(&self) -> Arc<HttpClientConfig> {
        self.http_config.clone()
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
