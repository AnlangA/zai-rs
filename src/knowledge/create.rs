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

/// Embedding model id enum mapped to integer ids
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmbeddingId {
    /// 3: Embedding-2
    Embedding2,
    /// 11: Embedding-3-new
    Embedding3New,
}

impl EmbeddingId {
    pub fn as_i64(&self) -> i64 {
        match self {
            EmbeddingId::Embedding2 => 3,
            EmbeddingId::Embedding3New => 11,
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
            other => Err(serde::de::Error::custom(format!(
                "unsupported embedding_id: {} (expected 3 or 11)",
                other
            ))),
        }
    }
}

/// Background color enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BackgroundColor {
    Blue,
    Red,
    Orange,
    Purple,
    Sky,
    Green,
    Yellow,
}

/// Knowledge icon enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum KnowledgeIcon {
    Question,
    Book,
    Seal,
    Wrench,
    Tag,
    Horn,
    House,
}

/// Request body for creating a knowledge base
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CreateKnowledgeBody {
    /// Embedding model id (3 or 11)
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
    pub fn new(key: String, embedding_id: EmbeddingId, name: impl Into<String>) -> Self {
        let endpoint_config = EndpointConfig::default();
        let api_base = ApiBase::LlmApplication;
        let url = endpoint_config.url(&api_base, paths::KNOWLEDGE);
        let body = CreateKnowledgeBody {
            embedding_id,
            name: name.into(),
            description: None,
            background: None,
            icon: None,
        };
        Self {
            key,
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

    pub fn with_base_url(mut self, base: impl Into<String>) -> Self {
        self.api_base = ApiBase::Custom(base.into());
        self.rebuild_url();
        self
    }

    pub fn with_endpoint_config(mut self, endpoint_config: EndpointConfig) -> Self {
        self.endpoint_config = endpoint_config;
        self.rebuild_url();
        self
    }

    pub fn with_http_config(mut self, config: HttpClientConfig) -> Self {
        self.http_config = Arc::new(config);
        self
    }

    /// Optional fields setters
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.body.description = Some(desc.into());
        self
    }
    pub fn with_background(mut self, bg: BackgroundColor) -> Self {
        self.body.background = Some(bg);
        self
    }
    pub fn with_icon(mut self, icon: KnowledgeIcon) -> Self {
        self.body.icon = Some(icon);
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

    fn api_url(&self) -> &Self::ApiUrl {
        &self.url
    }
    fn api_key(&self) -> &Self::ApiKey {
        &self.key
    }
    fn body(&self) -> &Self::Body {
        &self.body
    }

    fn http_config(&self) -> Arc<HttpClientConfig> {
        Arc::clone(&self.http_config)
    }
}

/// Response of knowledge creation
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CreateKnowledgeResponseData {
    /// Newly created id
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CreateKnowledgeResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<CreateKnowledgeResponseData>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<u64>,
}
