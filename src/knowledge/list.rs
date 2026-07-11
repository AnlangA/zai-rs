use std::sync::Arc;

use super::types::KnowledgeListResponse;
use crate::ZaiResult;
use crate::client::ZaiClient;
use crate::client::http::{HttpClientConfig, parse_typed_response, send_empty_request};

/// Query parameters for knowledge list API
#[derive(Debug, Clone, Default, serde::Serialize, validator::Validate)]
#[allow(clippy::new_without_default)]
pub struct KnowledgeListQuery {
    /// Page index starting from 1 (default 1)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(range(min = 1))]
    pub page: Option<u32>,
    /// Page size (default 10)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(range(min = 1))]
    pub size: Option<u32>,
}

impl KnowledgeListQuery {
    /// Create a new query (page 1, size 10).
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            page: Some(1),
            size: Some(10),
        }
    }
    /// Set the page index (1-based).
    pub fn with_page(mut self, page: u32) -> Self {
        self.page = Some(page);
        self
    }
    /// Set the page size.
    pub fn with_size(mut self, size: u32) -> Self {
        self.size = Some(size);
        self
    }

    /// Build the `(&str, String)` query pairs (in stable order) used to form
    /// the request URL.
    fn pairs(&self) -> Vec<(&'static str, String)> {
        let mut params: Vec<(&'static str, String)> = Vec::new();
        if let Some(page) = self.page.as_ref() {
            params.push(("page", page.to_string()));
        }
        if let Some(size) = self.size.as_ref() {
            params.push(("size", size.to_string()));
        }
        params
    }
}

/// Knowledge list request (GET /llm-application/open/knowledge)
///
/// Credentials and transport live on the [`ZaiClient`], passed to
/// [`send_via`](Self::send_via).
#[allow(clippy::new_without_default)]
pub struct KnowledgeListRequest {
    query: KnowledgeListQuery,
}

impl KnowledgeListRequest {
    /// Create a new knowledge-list request (default query: page 1, size 10).
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            query: KnowledgeListQuery::new(),
        }
    }

    /// Apply a query (replaces the current one).
    pub fn with_query(mut self, q: KnowledgeListQuery) -> Self {
        self.query = q;
        self
    }

    /// Send via a [`ZaiClient`] and parse the typed response.
    pub async fn send_via(&self, client: &ZaiClient) -> ZaiResult<KnowledgeListResponse> {
        let params = self.query.pairs();
        let url = client.endpoints().resolve_with_query(
            crate::client::ApiFamily::LlmApplication,
            &["knowledge"],
            &params
                .iter()
                .map(|(k, v)| (*k, v.as_str()))
                .collect::<Vec<_>>(),
        )?;
        let config = transport_config_from_client(client);
        let resp = send_empty_request(
            reqwest::Method::GET,
            url,
            client.secret().expose(),
            Arc::new(config),
        )
        .await?;
        parse_typed_response::<KnowledgeListResponse>(resp).await
    }

    /// Validate the query then send via a [`ZaiClient`] and parse the typed
    /// response.
    pub async fn send_via_with_query(
        mut self,
        client: &ZaiClient,
        q: &KnowledgeListQuery,
    ) -> ZaiResult<KnowledgeListResponse> {
        use validator::Validate;
        q.validate()?;
        self.query = q.clone();
        self.send_via(client).await
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
