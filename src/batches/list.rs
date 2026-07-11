use serde::{Deserialize, Serialize};
use validator::Validate;

use super::types::BatchItem;
use crate::{ZaiResult, client::ZaiClient};

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
        let route = crate::client::routes::BATCHES_LIST;
        let url = client
            .endpoints()
            .resolve_route_with_query(route, &[], &borrowed)?;
        client
            .send_empty::<BatchesListResponse>(route.method(), url)
            .await
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
