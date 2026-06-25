use std::sync::Arc;

use serde::{Deserialize, Serialize};
use validator::Validate;

use super::types::BatchItem;
use crate::{
    ZaiResult,
    client::{
        endpoints::{ApiBase, EndpointConfig, build_query, paths},
        http::{HttpClient, HttpClientConfig, parse_typed_response},
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
    /// Bearer API key
    pub key: String,
    /// Fully built request URL (with query string)
    url: String,
    endpoint_config: EndpointConfig,
    api_base: ApiBase,
    http_config: Arc<HttpClientConfig>,
    query: BatchesListQuery,
    /// No body for GET
    _body: (),
}

impl BatchesListRequest {
    /// Create a request targeting the batches list endpoint
    pub fn new(key: impl Into<String>) -> Self {
        let endpoint_config = EndpointConfig::default();
        let api_base = ApiBase::PaasV4;
        let url = endpoint_config.url(&api_base, paths::BATCHES);
        Self {
            key: key.into(),
            url,
            endpoint_config,
            api_base,
            http_config: Arc::new(HttpClientConfig::default()),
            query: BatchesListQuery::new(),
            _body: (),
        }
    }

    /// Rebuild URL with query parameters
    fn rebuild_url(&mut self) {
        let endpoint = self.endpoint_config.url(&self.api_base, paths::BATCHES);
        let mut params: Vec<(&str, String)> = Vec::new();
        if let Some(after) = self.query.after.as_ref() {
            params.push(("after", after.clone()));
        }
        if let Some(limit) = self.query.limit.as_ref() {
            params.push(("limit", limit.to_string()));
        }
        self.url = build_query(&endpoint, params);
    }

    /// Override the base URL (uses [`ApiBase::Custom`]).
    pub fn with_base_url(mut self, base: impl Into<String>) -> Self {
        self.api_base = ApiBase::Custom(base.into());
        self.rebuild_url();
        self
    }

    /// Replace the full [`EndpointConfig`] used to resolve URLs.
    pub fn with_endpoint_config(mut self, endpoint_config: EndpointConfig) -> Self {
        self.endpoint_config = endpoint_config;
        self.rebuild_url();
        self
    }

    /// Replace the HTTP client configuration (timeouts, retries, …).
    pub fn with_http_config(mut self, config: HttpClientConfig) -> Self {
        self.http_config = Arc::new(config);
        self
    }

    /// Attach a query to this request
    pub fn with_query(mut self, q: BatchesListQuery) -> Self {
        self.query = q;
        self.rebuild_url();
        self
    }

    /// Send the request and parse typed response.
    pub async fn send(&self) -> ZaiResult<BatchesListResponse> {
        let resp: reqwest::Response = self.get().await?;

        let parsed = parse_typed_response::<BatchesListResponse>(resp).await?;

        Ok(parsed)
    }
}

impl HttpClient for BatchesListRequest {
    type Body = ();
    type ApiUrl = String;
    type ApiKey = String;

    /// Resolved target URL (with query string) for the request.
    fn api_url(&self) -> &Self::ApiUrl {
        &self.url
    }
    /// API key used for `Authorization: Bearer …`.
    fn api_key(&self) -> &Self::ApiKey {
        &self.key
    }
    /// Empty body placeholder (GET request).
    fn body(&self) -> &Self::Body {
        &self._body
    }

    /// HTTP client configuration (timeouts, retries, …).
    fn http_config(&self) -> Arc<HttpClientConfig> {
        Arc::clone(&self.http_config)
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
