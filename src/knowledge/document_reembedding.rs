use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use validator::Validate;

use super::types::KnowledgeOperationResponse;
use crate::ZaiResult;
use crate::client::ZaiClient;

/// Request body for re-embedding a document
#[derive(Clone, Serialize, Deserialize, Validate, Default)]
pub struct DocumentReembedBody {
    /// Optional callback URL that will be called when re-embedding completes
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(url)]
    pub callback_url: Option<String>,
    /// Optional callback headers key-value pairs
    #[serde(skip_serializing_if = "Option::is_none")]
    pub callback_header: Option<BTreeMap<String, String>>,
}

impl std::fmt::Debug for DocumentReembedBody {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("DocumentReembedBody")
            .field("callback_url_configured", &self.callback_url.is_some())
            .field(
                "callback_header_entries",
                &self.callback_header.as_ref().map(BTreeMap::len),
            )
            .finish()
    }
}

/// Re-embedding request (POST /llm-application/open/document/embedding/{id})
///
/// Credentials and transport live on the [`ZaiClient`], passed to
/// [`send_via`](Self::send_via).
pub struct DocumentReembedRequest {
    document_id: String,
    body: DocumentReembedBody,
}

impl DocumentReembedRequest {
    /// Create a new request for the specified document id
    pub fn new(document_id: impl Into<String>) -> Self {
        Self {
            document_id: document_id.into(),
            body: DocumentReembedBody::default(),
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

    /// Validate the target identifier and optional callback URL without I/O.
    pub fn validate(&self) -> ZaiResult<()> {
        crate::client::validation::require_non_blank(&self.document_id, "document_id")?;
        self.body.validate()?;
        Ok(())
    }

    /// Send the POST request via a [`ZaiClient`] and parse the typed response.
    pub async fn send_via(&self, client: &ZaiClient) -> ZaiResult<DocumentReembedResponse> {
        self.validate()?;
        let route = crate::client::routes::DOCUMENTS_REEMBED;
        if self.body.callback_url.is_none() && self.body.callback_header.is_none() {
            client
                .operation(route)
                .with_parameters([self.document_id.as_str()])
                .send_empty::<DocumentReembedResponse>()
                .await
        } else {
            client
                .operation(route)
                .with_parameters([self.document_id.as_str()])
                .send_json::<_, DocumentReembedResponse>(&self.body)
                .await
        }
    }
}

/// Simple response envelope without a data payload.
pub type DocumentReembedResponse = KnowledgeOperationResponse;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_identifier_and_callback_url() {
        assert!(DocumentReembedRequest::new(" ").validate().is_err());
        assert!(
            DocumentReembedRequest::new("doc-1")
                .with_callback_url("not a URL")
                .validate()
                .is_err()
        );
        assert!(DocumentReembedRequest::new("doc-1").validate().is_ok());
    }

    #[test]
    fn body_debug_redacts_callback_url_and_headers() {
        let body = DocumentReembedBody {
            callback_url: Some("https://private.example/callback".to_owned()),
            callback_header: Some(BTreeMap::from([(
                "Authorization".to_owned(),
                "private-token".to_owned(),
            )])),
        };
        let debug = format!("{body:?}");
        for secret in ["private.example", "Authorization", "private-token"] {
            assert!(!debug.contains(secret));
        }
        assert!(debug.contains("callback_header_entries: Some(1)"));
    }
}
