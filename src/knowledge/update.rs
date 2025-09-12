use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use validator::Validate;

use super::create::{BackgroundColor, EmbeddingId, KnowledgeIcon};
use crate::client::http::HttpClient;

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
    body: UpdateKnowledgeBody,
}

impl KnowledgeUpdateRequest {
    /// Build update request targeting a specific id with empty body
    pub fn new(key: String, id: impl AsRef<str>) -> Self {
        let url = format!(
            "https://open.bigmodel.cn/api/llm-application/open/knowledge/{}",
            id.as_ref()
        );
        Self {
            key,
            url,
            body: UpdateKnowledgeBody::default(),
        }
    }

    /// Setters to update individual fields
    pub fn with_embedding_id(mut self, id: EmbeddingId) -> Self {
        self.body.embedding_id = Some(id);
        self
    }
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.body.name = Some(name.into());
        self
    }
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
    pub fn with_callback_url(mut self, url: impl Into<String>) -> Self {
        self.body.callback_url = Some(url.into());
        self
    }
    pub fn with_callback_header(mut self, headers: HashMap<String, String>) -> Self {
        self.body.callback_header = Some(headers);
        self
    }

    /// Validate and send the update request. Requires at least one field set.
    pub async fn send(&self) -> anyhow::Result<KnowledgeUpdateResponse> {
        if self.body.is_empty() {
            return Err(anyhow::anyhow!(
                "update body is empty; set at least one field"
            ));
        }
        self.body.validate()?;
        let resp = self.put().await?;
        let parsed = resp.json::<KnowledgeUpdateResponse>().await?;
        Ok(parsed)
    }

    /// Perform HTTP PUT with JSON body and error handling similar to other clients
    pub fn put(
        &self,
    ) -> impl std::future::Future<Output = anyhow::Result<reqwest::Response>> + Send {
        let url = self.url.clone();
        let key = self.key.clone();
        let body = self.body.clone();
        async move {
            let body_str = serde_json::to_string(&body)?;
            let resp = reqwest::Client::new()
                .put(url)
                .bearer_auth(key)
                .header("Content-Type", "application/json")
                .body(body_str)
                .send()
                .await?;
            let status = resp.status();
            if status.is_success() {
                return Ok(resp);
            }
            // Non-success: try parse standard error envelope {"error": {code, message}}
            let text = resp.text().await.unwrap_or_default();
            #[derive(serde::Deserialize)]
            struct ErrEnv {
                error: ErrObj,
            }
            #[derive(serde::Deserialize)]
            struct ErrObj {
                code: serde_json::Value,
                message: String,
            }
            if let Ok(parsed) = serde_json::from_str::<ErrEnv>(&text) {
                return Err(anyhow::anyhow!(
                    "HTTP {} {} | code={} | message={}",
                    status.as_u16(),
                    status.canonical_reason().unwrap_or(""),
                    parsed.error.code,
                    parsed.error.message
                ));
            }
            Err(anyhow::anyhow!(
                "HTTP {} {} | body={}",
                status.as_u16(),
                status.canonical_reason().unwrap_or(""),
                text
            ))
        }
    }
}

impl HttpClient for KnowledgeUpdateRequest {
    type Body = UpdateKnowledgeBody;
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
}

/// Update response envelope without data
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct KnowledgeUpdateResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<u64>,
}
