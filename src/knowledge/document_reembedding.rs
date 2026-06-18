use std::{collections::BTreeMap, sync::Arc};

use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::{
    ZaiResult,
    client::{
        endpoints::{ApiBase, EndpointConfig, join_url, paths},
        http::{HttpClient, HttpClientConfig, parse_typed_response},
    },
};

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
    endpoint_config: EndpointConfig,
    api_base: ApiBase,
    document_id: String,
    http_config: Arc<HttpClientConfig>,
    body: DocumentReembeddingBody,
}

impl DocumentReembeddingRequest {
    /// Create a new request for the specified document id
    pub fn new(key: String, document_id: impl AsRef<str>) -> Self {
        let document_id = document_id.as_ref().to_string();
        let endpoint_config = EndpointConfig::default();
        let api_base = ApiBase::LlmApplication;
        let url = endpoint_config.url(
            &api_base,
            &join_url(paths::DOCUMENT_EMBEDDING, &document_id),
        );
        Self {
            key,
            url,
            endpoint_config,
            api_base,
            document_id,
            http_config: Arc::new(HttpClientConfig::default()),
            body: DocumentReembeddingBody::default(),
        }
    }

    fn rebuild_url(&mut self) {
        self.url = self.endpoint_config.url(
            &self.api_base,
            &join_url(paths::DOCUMENT_EMBEDDING, &self.document_id),
        );
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
        let parsed = parse_typed_response::<DocumentReembeddingResponse>(resp).await?;
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

    fn http_config(&self) -> Arc<HttpClientConfig> {
        Arc::clone(&self.http_config)
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
