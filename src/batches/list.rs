use serde::{Deserialize, Serialize};
use url::Url;
use validator::Validate;

use crate::client::http::HttpClient;

use super::types::BatchItem;

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
    /// No body for GET
    _body: (),
}

impl BatchesListRequest {
    /// Create a request targeting the batches list endpoint
    pub fn new(key: String) -> Self {
        let url = "https://open.bigmodel.cn/api/paas/v4/batches".to_string();
        Self {
            key,
            url,
            _body: (),
        }
    }

    /// Rebuild URL with query parameters
    fn rebuild_url(&mut self, q: &BatchesListQuery) {
        let mut url = Url::parse("https://open.bigmodel.cn/api/paas/v4/batches").unwrap();
        {
            let mut pairs = url.query_pairs_mut();
            if let Some(after) = q.after.as_ref() {
                pairs.append_pair("after", after);
            }
            if let Some(limit) = q.limit.as_ref() {
                pairs.append_pair("limit", &limit.to_string());
            }
        }
        self.url = url.to_string();
    }

    /// Attach a query to this request
    pub fn with_query(mut self, q: BatchesListQuery) -> Self {
        self.rebuild_url(&q);
        self
    }

    /// Send the request and parse typed response.
    pub async fn send(&self) -> anyhow::Result<BatchesListResponse> {
        let resp: reqwest::Response = self.get().await?;
        let parsed = resp.json::<BatchesListResponse>().await?;
        Ok(parsed)
    }
}

impl HttpClient for BatchesListRequest {
    type Body = ();
    type ApiUrl = String;
    type ApiKey = String;

    fn api_url(&self) -> &Self::ApiUrl {
        &self.url
    }
    fn api_key(&self) -> &Self::ApiKey {
        &self.key
    }
    fn body(&self) -> &Self::Body {
        &self._body
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
