use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::{ZaiResult, client::http::HttpClient};

/// Request body for re-embedding a document
#[derive(Debug, Clone, Serialize, Deserialize, Validate, Default)]
pub struct DocumentReembeddingBody {
    /// Optional callback URL that will be called when re-embedding completes
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(url)]
    pub callback_url: Option<String>,
    /// Optional callback headers key-value pairs
    #[serde(skip_serializing_if = "Option::is_none")]
    pub callback_header: Option<BTreeMap<String, String>>,
}

/// Re-embedding request (POST /llm-application/open/document/embedding/{id})
pub struct DocumentReembeddingRequest {
    /// Bearer API key
    pub key: String,
    url: String,
    body: DocumentReembeddingBody,
}

impl DocumentReembeddingRequest {
    /// Create a new request for the specified document id
    pub fn new(key: String, document_id: impl AsRef<str>) -> Self {
        let url = format!(
            "https://open.bigmodel.cn/api/llm-application/open/document/embedding/{}",
            document_id.as_ref()
        );
        Self {
            key,
            url,
            body: DocumentReembeddingBody::default(),
        }
    }

    /// Set callback URL
    pub fn with_callback_url(mut self, url: impl Into<String>) -> Self {
        self.body.callback_url = Some(url.into());
        self
    }

    /// Set callback headers
    pub fn with_callback_header(mut self, hdr: BTreeMap<String, String>) -> Self {
        self.body.callback_header = Some(hdr);
        self
    }

    /// Send POST request with JSON body and parse typed response
    pub async fn send(&self) -> ZaiResult<DocumentReembeddingResponse> {
        // validate body
        self.body.validate()?;
        let resp = self.post().await?;
        let parsed = resp.json::<DocumentReembeddingResponse>().await?;
        Ok(parsed)
    }
}

impl HttpClient for DocumentReembeddingRequest {
    type Body = DocumentReembeddingBody;
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

/// Simple response envelope: { code, message, timestamp }
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct DocumentReembeddingResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<u64>,
}
