//! Knowledge search (POST /llm-application/open/knowledge/retrieve) — plan P06.
use std::sync::Arc;

use crate::ZaiResult;
use crate::client::http::{HttpClientConfig, parse_typed_response, send_json_request};
use crate::client::v2::{ApiFamily, ZaiClient};
use serde::Serialize;

/// POST /knowledge/retrieve search request (P06 knowledge.retrieve).
#[derive(Debug, Clone, Serialize)]
pub struct KnowledgeSearchBody {
    /// Knowledge base id.
    pub knowledge_id: String,
    /// Search query text.
    pub query: String,
    /// Top-k results (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<u32>,
    /// Score threshold (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score_threshold: Option<f64>,
}

pub struct KnowledgeSearchRequest {
    pub body: KnowledgeSearchBody,
}

impl KnowledgeSearchRequest {
    pub fn new(knowledge_id: impl Into<String>, query: impl Into<String>) -> Self {
        Self {
            body: KnowledgeSearchBody {
                knowledge_id: knowledge_id.into(),
                query: query.into(),
                top_k: None,
                score_threshold: None,
            },
        }
    }

    pub fn with_top_k(mut self, top_k: u32) -> Self {
        self.body.top_k = Some(top_k);
        self
    }

    pub fn with_score_threshold(mut self, threshold: f64) -> Self {
        self.body.score_threshold = Some(threshold);
        self
    }

    pub async fn send_via(&self, client: &ZaiClient) -> ZaiResult<KnowledgeSearchResponse> {
        let url = client
            .endpoints()
            .resolve(ApiFamily::LlmApplication, &["knowledge", "retrieve"])?;
        let config = transport_config(client);
        let resp = send_json_request(
            reqwest::Method::POST,
            url,
            client.secret().expose(),
            &self.body,
            Arc::new(config),
        )
        .await?;
        parse_typed_response::<KnowledgeSearchResponse>(resp).await
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct KnowledgeSearchResponse {
    #[serde(default)]
    pub data: Vec<serde_json::Value>,
}

fn transport_config(client: &ZaiClient) -> HttpClientConfig {
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
