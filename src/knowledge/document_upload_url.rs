use crate::client::http::HttpClient;
use serde::{Deserialize, Serialize};
use validator::Validate;

use super::types::UploadUrlResponse;

/// Single URL upload detail
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct UploadUrlDetail {
    /// Source URL to fetch
    #[validate(url)]
    pub url: String,
    /// Slice type (integer)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub knowledge_type: Option<i64>,
    /// Custom separators
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_separator: Option<Vec<String>>,
    /// Sentence size
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(range(min = 1))]
    pub sentence_size: Option<u32>,
    /// Callback URL
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(url)]
    pub callback_url: Option<String>,
    /// Callback headers
    #[serde(skip_serializing_if = "Option::is_none")]
    pub callback_header: Option<std::collections::BTreeMap<String, String>>,
}

impl UploadUrlDetail {
    pub fn new(url: impl Into<String>) -> Self {
        Self { url: url.into(), knowledge_type: None, custom_separator: None, sentence_size: None, callback_url: None, callback_header: None }
    }
    pub fn with_knowledge_type(mut self, t: i64) -> Self { self.knowledge_type = Some(t); self }
    pub fn with_custom_separator(mut self, seps: Vec<String>) -> Self { self.custom_separator = Some(seps); self }
    pub fn with_sentence_size(mut self, size: u32) -> Self { self.sentence_size = Some(size); self }
    pub fn with_callback_url(mut self, url: impl Into<String>) -> Self { self.callback_url = Some(url.into()); self }
    pub fn with_callback_header(mut self, headers: std::collections::BTreeMap<String, String>) -> Self { self.callback_header = Some(headers); self }
}

/// Upload URL request body
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct UploadUrlBody {
    /// Upload detail list (at least 1)
    #[validate(length(min = 1))]
    pub upload_detail: Vec<UploadUrlDetail>,
    /// Knowledge base id
    #[validate(length(min = 1))]
    pub knowledge_id: String,
}

impl UploadUrlBody {
    pub fn new(knowledge_id: impl Into<String>) -> Self { Self { upload_detail: Vec::new(), knowledge_id: knowledge_id.into() } }
    pub fn add_detail(mut self, detail: UploadUrlDetail) -> Self { self.upload_detail.push(detail); self }
    pub fn add_url(mut self, url: impl Into<String>) -> Self { self.upload_detail.push(UploadUrlDetail::new(url)); self }
}

/// Upload URL request (POST /llm-application/open/document/upload_url)
pub struct DocumentUploadUrlRequest {
    /// Bearer API key
    pub key: String,
    url: String,
    body: UploadUrlBody,
}

impl DocumentUploadUrlRequest {
    pub fn new(key: String, body: UploadUrlBody) -> Self {
        let url = "https://open.bigmodel.cn/api/llm-application/open/document/upload_url".to_string();
        Self { key, url, body }
    }

    pub fn body_mut(&mut self) -> &mut UploadUrlBody { &mut self.body }

    /// Validate and send
    pub async fn send(&self) -> anyhow::Result<UploadUrlResponse> {
        self.body.validate()?;
        let resp = self.post().await?;
        let parsed = resp.json::<UploadUrlResponse>().await?;
        Ok(parsed)
    }
}

impl HttpClient for DocumentUploadUrlRequest {
    type Body = UploadUrlBody;
    type ApiUrl = String;
    type ApiKey = String;

    fn api_url(&self) -> &Self::ApiUrl { &self.url }
    fn api_key(&self) -> &Self::ApiKey { &self.key }
    fn body(&self) -> &Self::Body { &self.body }
}

