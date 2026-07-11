use std::sync::Arc;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use validator::Validate;

use crate::{
    ZaiResult,
    client::{
        endpoints::{ApiBase, EndpointConfig, paths},
        http::{HttpClient, HttpClientConfig, parse_typed_response},
    },
};

/// Embedding model id enum mapped to integer ids (plan §13.5).
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
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CreateKnowledgeBody {
    /// Embedding model id (3, 11 or 12; plan §13.5).
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
    /// the two must be consistent (plan §13.5).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding_model: Option<String>,
    /// Contextual retrieval flag (0/1; plan §13.5).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contextual: Option<u8>,
}

/// Create knowledge request (POST /llm-application/open/knowledge)
pub struct CreateKnowledgeRequest {
    /// Bearer key
    pub key: String,
    url: String,
    endpoint_config: EndpointConfig,
    api_base: ApiBase,
    http_config: Arc<HttpClientConfig>,
    body: CreateKnowledgeBody,
}

impl CreateKnowledgeRequest {
    /// Build a create request with required fields
    pub fn new(key: impl Into<String>, embedding_id: EmbeddingId, name: impl Into<String>) -> Self {
        let endpoint_config = EndpointConfig::default();
        let api_base = ApiBase::LlmApplication;
        let url = endpoint_config.url(&api_base, paths::KNOWLEDGE);
        let body = CreateKnowledgeBody {
            embedding_id,
            name: name.into(),
            description: None,
            background: None,
            icon: None,
            embedding_model: None,
            contextual: None,
        };
        Self {
            key: key.into(),
            url,
            endpoint_config,
            api_base,
            http_config: Arc::new(HttpClientConfig::default()),
            body,
        }
    }

    fn rebuild_url(&mut self) {
        self.url = self.endpoint_config.url(&self.api_base, paths::KNOWLEDGE);
    }

    /// Override the base URL (uses [`ApiBase::Custom`]).
    pub fn with_base_url(mut self, base: impl Into<String>) -> Self {
        self.api_base = ApiBase::Custom(base.into());
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

    /// Optional fields setters
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
    /// Set the embedding model name (optional). When given alongside
    /// `embedding_id`, the two must be consistent (plan §13.5).
    pub fn with_embedding_model(mut self, model: impl Into<String>) -> Self {
        self.body.embedding_model = Some(model.into());
        self
    }
    /// Set the contextual-retrieval flag (0 or 1; plan §13.5).
    pub fn with_contextual(mut self, contextual: u8) -> Self {
        self.body.contextual = Some(contextual);
        self
    }

    /// Validate and send, returning typed response
    pub async fn send(&self) -> ZaiResult<CreateKnowledgeResponse> {
        self.body.validate()?;

        let resp: reqwest::Response = self.post().await?;

        let parsed = parse_typed_response::<CreateKnowledgeResponse>(resp).await?;
        Ok(parsed)
    }
}

impl HttpClient for CreateKnowledgeRequest {
    type Body = CreateKnowledgeBody;
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
        Arc::clone(&self.http_config)
    }
}

/// Inner data of [`CreateKnowledgeResponse`] — the newly created id.
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CreateKnowledgeResponseData {
    /// Newly created id
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
}

/// Response of knowledge creation
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CreateKnowledgeResponse {
    /// Created knowledge-base data.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<CreateKnowledgeResponseData>,
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
