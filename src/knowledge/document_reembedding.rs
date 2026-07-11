use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::ZaiResult;
use crate::client::ZaiClient;

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
///
/// Credentials and transport live on the [`ZaiClient`], passed to
/// [`send_via`](Self::send_via).
pub struct DocumentReembeddingRequest {
    document_id: String,
    body: DocumentReembeddingBody,
}

impl DocumentReembeddingRequest {
    /// Create a new request for the specified document id
    pub fn new(document_id: impl Into<String>) -> Self {
        Self {
            document_id: document_id.into(),
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

    /// Send the POST request via a [`ZaiClient`] and parse the typed response.
    pub async fn send_via(&self, client: &ZaiClient) -> ZaiResult<DocumentReembeddingResponse> {
        self.body.validate()?;
        let url = client.endpoints().resolve(
            crate::client::ApiFamily::LlmApplication,
            &["document", "embedding", &self.document_id],
        )?;
        client
            .send_json::<_, DocumentReembeddingResponse>("POST", url, &self.body)
            .await
    }
}

/// Simple response envelope: { code, message, timestamp }
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct DocumentReembeddingResponse {
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
