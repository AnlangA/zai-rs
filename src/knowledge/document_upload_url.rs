use std::sync::Arc;

use serde::{Deserialize, Serialize};
use validator::Validate;

use super::types::UploadUrlResponse;
use crate::ZaiResult;
use crate::client::http::{HttpClientConfig, parse_typed_response, send_json_request};
use crate::client::v2::ZaiClient;

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
    /// Create a new upload detail for the given source URL.
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            knowledge_type: None,
            custom_separator: None,
            sentence_size: None,
            callback_url: None,
            callback_header: None,
        }
    }
    /// Set the slice/knowledge type.
    pub fn with_knowledge_type(mut self, t: i64) -> Self {
        self.knowledge_type = Some(t);
        self
    }
    /// Set custom separators.
    pub fn with_custom_separator(mut self, seps: Vec<String>) -> Self {
        self.custom_separator = Some(seps);
        self
    }
    /// Set the sentence size.
    pub fn with_sentence_size(mut self, size: u32) -> Self {
        self.sentence_size = Some(size);
        self
    }
    /// Set the callback URL.
    pub fn with_callback_url(mut self, url: impl Into<String>) -> Self {
        self.callback_url = Some(url.into());
        self
    }
    /// Set the callback headers.
    pub fn with_callback_header(
        mut self,
        headers: std::collections::BTreeMap<String, String>,
    ) -> Self {
        self.callback_header = Some(headers);
        self
    }
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
    /// Create a new upload body for the given knowledge base id.
    pub fn new(knowledge_id: impl Into<String>) -> Self {
        Self {
            upload_detail: Vec::new(),
            knowledge_id: knowledge_id.into(),
        }
    }
    /// Append a fully-built [`UploadUrlDetail`].
    pub fn add_detail(mut self, detail: UploadUrlDetail) -> Self {
        self.upload_detail.push(detail);
        self
    }
    /// Convenience: append a detail built from a single URL.
    pub fn add_url(mut self, url: impl Into<String>) -> Self {
        self.upload_detail.push(UploadUrlDetail::new(url));
        self
    }
}

/// Upload URL request (POST /llm-application/open/document/upload_url)
///
/// Credentials and transport live on the [`ZaiClient`], passed to
/// [`send_via`](Self::send_via).
pub struct DocumentUploadUrlRequest {
    body: UploadUrlBody,
}

impl DocumentUploadUrlRequest {
    /// Create a new upload-by-URL request with the given body.
    pub fn new(body: UploadUrlBody) -> Self {
        Self { body }
    }

    /// Borrow the underlying [`UploadUrlBody`] mutably.
    pub fn body_mut(&mut self) -> &mut UploadUrlBody {
        &mut self.body
    }

    /// Validate and send via a [`ZaiClient`], returning the typed response.
    pub async fn send_via(&self, client: &ZaiClient) -> ZaiResult<UploadUrlResponse> {
        self.body.validate()?;
        let url = client.endpoints().resolve(
            crate::client::v2::ApiFamily::LlmApplication,
            &["document", "upload_url"],
        )?;
        let config = transport_config_from_client(client);
        let resp = send_json_request(
            reqwest::Method::POST,
            url,
            client.secret().expose(),
            &self.body,
            Arc::new(config),
        )
        .await?;
        parse_typed_response::<UploadUrlResponse>(resp).await
    }
}

fn transport_config_from_client(client: &ZaiClient) -> HttpClientConfig {
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
