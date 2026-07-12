use serde::{Deserialize, Deserializer, Serialize, de::Error as _};
use validator::Validate;

use super::types::BatchItem;
use crate::{ZaiResult, client::ZaiClient};

/// Query parameters for listing batch processing tasks
#[derive(Clone, Serialize, Deserialize, Validate)]
pub struct BatchListQuery {
    /// Pagination cursor: return results after this ID
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(length(min = 1))]
    pub after: Option<String>,

    /// Page size (server default 20).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

impl std::fmt::Debug for BatchListQuery {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("BatchListQuery")
            .field("after", &self.after.as_ref().map(|_| "[REDACTED]"))
            .field("limit", &self.limit)
            .finish()
    }
}

impl Default for BatchListQuery {
    fn default() -> Self {
        Self::new()
    }
}

impl BatchListQuery {
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

    /// Set the page size.
    pub fn with_limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }
}

/// Batches list request (GET /paas/v4/batches)
pub struct BatchListRequest {
    query: BatchListQuery,
}

impl BatchListRequest {
    /// Create a request targeting the batches list endpoint
    pub fn new() -> Self {
        Self {
            query: BatchListQuery::new(),
        }
    }

    /// Attach a query to this request
    pub fn with_query(mut self, q: BatchListQuery) -> Self {
        self.query = q;
        self
    }

    /// Validate the query, send the request, and parse the typed response.
    pub async fn send_via(&self, client: &ZaiClient) -> ZaiResult<BatchListResponse> {
        self.query.validate()?;
        if self
            .query
            .after
            .as_deref()
            .is_some_and(|after| after.trim().is_empty())
        {
            return Err(crate::client::validation::invalid(
                "after cannot be blank when provided",
            ));
        }
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
            .send_empty::<BatchListResponse>(route.method(), url)
            .await
    }
}

impl Default for BatchListRequest {
    fn default() -> Self {
        Self::new()
    }
}

/// Response for listing batch processing tasks
#[derive(Debug, Clone, Serialize, Validate)]
pub struct BatchListResponse {
    /// Response type ("list")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object: Option<BatchListObject>,

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

#[derive(Deserialize)]
struct BatchListResponseWire {
    object: Option<BatchListObject>,
    data: Option<Vec<BatchItem>>,
    first_id: Option<String>,
    last_id: Option<String>,
    has_more: Option<bool>,
}

impl<'de> Deserialize<'de> for BatchListResponse {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = BatchListResponseWire::deserialize(deserializer)?;
        if wire.object.is_none()
            && wire.data.is_none()
            && wire.first_id.is_none()
            && wire.last_id.is_none()
            && wire.has_more.is_none()
        {
            return Err(D::Error::custom(
                "batch-list response contained no documented non-null fields",
            ));
        }
        Ok(Self {
            object: wire.object,
            data: wire.data,
            first_id: wire.first_id,
            last_id: wire.last_id,
            has_more: wire.has_more,
        })
    }
}

/// Object type for list responses
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BatchListObject {
    /// List marker
    List,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_response_requires_a_documented_non_null_field() {
        assert!(serde_json::from_str::<BatchListResponse>("{}").is_err());
        assert!(serde_json::from_str::<BatchListResponse>(r#"{"data":null}"#).is_err());
        assert!(serde_json::from_str::<BatchListResponse>(r#"{"data":[]}"#).is_ok());
        assert!(serde_json::from_str::<BatchListResponse>(r#"{"object":"future"}"#).is_err());
    }

    #[test]
    fn query_debug_redacts_the_pagination_identifier() {
        let query = BatchListQuery::new().with_after("private-batch-id");
        assert!(!format!("{query:?}").contains("private-batch-id"));
    }

    #[test]
    fn limit_does_not_invent_an_upstream_range() {
        assert!(BatchListQuery::new().with_limit(0).validate().is_ok());
    }
}
