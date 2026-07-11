use std::sync::Arc;

use serde::{Deserialize, Serialize};
use validator::Validate;

use super::types::BatchItem;
use crate::{
    ZaiResult,
    client::{
        http::{HttpClientConfig, parse_typed_response, send_empty_request},
        {ApiFamily, ZaiClient},
    },
};

/// Query parameters for listing batch processing tasks
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct BatchesListQuery {
    /// Pagination cursor: return results after this ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after: Option<String>,

    /// Page size 1..=100 (server default 20)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(range(min = 1, max = 100))]
    pub limit: Option<u32>,
}

impl Default for BatchesListQuery {
    fn default() -> Self {
        Self::new()
    }
}

impl BatchesListQuery {
    /// Create an empty query (no filters)
    pub fn new() -> Self {
        Self {
            after: None,
            limit: None,
        }
    }

    /// Set the pagination cursor
    pub fn with_after(mut self, after: impl Into<String>) -> Self {
        self.after = Some(after.into());
        self
    }

    /// Set page size (1..=100)
    pub fn with_limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }
}

/// Batches list request (GET /paas/v4/batches)
pub struct BatchesListRequest {
    query: BatchesListQuery,
}

impl BatchesListRequest {
    /// Create a request targeting the batches list endpoint
    pub fn new() -> Self {
        Self {
            query: BatchesListQuery::new(),
        }
    }

    /// Attach a query to this request
    pub fn with_query(mut self, q: BatchesListQuery) -> Self {
        self.query = q;
        self
    }

    /// Send the request via a [`ZaiClient`] and parse typed response.
    pub async fn send_via(&self, client: &ZaiClient) -> ZaiResult<BatchesListResponse> {
        let mut params: Vec<(&str, String)> = Vec::new();
        if let Some(after) = self.query.after.as_ref() {
            params.push(("after", after.clone()));
        }
        if let Some(limit) = self.query.limit.as_ref() {
            params.push(("limit", limit.to_string()));
        }
        let borrowed: Vec<(&str, &str)> = params.iter().map(|(k, v)| (*k, v.as_str())).collect();
        let url =
            client
                .endpoints()
                .resolve_with_query(ApiFamily::PaasV4, &["batches"], &borrowed)?;
        let config = transport_config_from_client(client);
        let resp: reqwest::Response = send_empty_request(
            reqwest::Method::GET,
            url,
            client.secret().expose(),
            Arc::new(config),
        )
        .await?;

        let parsed = parse_typed_response::<BatchesListResponse>(resp).await?;

        Ok(parsed)
    }
}

impl Default for BatchesListRequest {
    fn default() -> Self {
        Self::new()
    }
}

/// Response for listing batch processing tasks
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct BatchesListResponse {
    /// Response type ("list")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object: Option<ListObject>,

    /// Batch task entries
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Vec<BatchItem>>,

    /// First ID in this page
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_id: Option<String>,

    /// Last ID in this page
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_id: Option<String>,

    /// Whether more data is available
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_more: Option<bool>,
}

/// Object type for list responses
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ListObject {
    /// List marker
    List,
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
