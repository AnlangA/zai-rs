use serde::{Deserialize, Deserializer, Serialize, de::Error as _};
use validator::Validate;

use super::types::BatchItem;
use crate::{ZaiResult, client::ZaiClient, pagination::CursorPagination};

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

    /// Replace the cursor and limit with validated pagination values.
    ///
    /// This safe path requires a non-zero limit. The existing
    /// [`with_limit`](Self::with_limit) method retains its legacy behavior.
    pub fn try_with_pagination(mut self, pagination: CursorPagination) -> ZaiResult<Self> {
        let (after, limit) = pagination.into_parts();
        self.after = after;
        self.limit = limit;
        Ok(self)
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

    /// Replace the request's cursor and limit with validated pagination.
    pub fn try_with_pagination(mut self, pagination: CursorPagination) -> ZaiResult<Self> {
        self.query = self.query.try_with_pagination(pagination)?;
        Ok(self)
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
        let route = crate::client::routes::BATCHES_LIST;
        client
            .operation(route)
            .with_query(&self.query)?
            .send_empty::<BatchListResponse>()
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
    /// Response type ("list"). An unknown string marker maps to `None` while
    /// the remaining documented payload is preserved.
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

impl BatchListResponse {
    /// Return the cursor for the next page when the response explicitly says
    /// more data is available and supplies a non-blank last ID.
    pub fn next_after(&self) -> Option<&str> {
        if self.has_more != Some(true) {
            return None;
        }
        self.last_id
            .as_deref()
            .filter(|last_id| !last_id.trim().is_empty())
    }
}

#[derive(Deserialize)]
struct BatchListResponseWire {
    #[serde(default, deserialize_with = "deserialize_optional_batch_list_object")]
    object: Option<BatchListObject>,
    data: Option<Vec<BatchItem>>,
    first_id: Option<String>,
    last_id: Option<String>,
    has_more: Option<bool>,
}

enum BatchListObjectWire {
    List,
    Unknown,
}

impl<'de> Deserialize<'de> for BatchListObjectWire {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct MarkerVisitor;

        impl serde::de::Visitor<'_> for MarkerVisitor {
            type Value = BatchListObjectWire;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("a batch-list object marker string")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(match value {
                    "list" => BatchListObjectWire::List,
                    _ => BatchListObjectWire::Unknown,
                })
            }
        }

        deserializer.deserialize_str(MarkerVisitor)
    }
}

fn deserialize_optional_batch_list_object<'de, D>(
    deserializer: D,
) -> Result<Option<BatchListObject>, D::Error>
where
    D: Deserializer<'de>,
{
    let marker = Option::<BatchListObjectWire>::deserialize(deserializer)?;
    Ok(match marker {
        Some(BatchListObjectWire::List) => Some(BatchListObject::List),
        Some(BatchListObjectWire::Unknown) | None => None,
    })
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

/// Known object type for list responses.
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
        assert!(
            serde_json::from_str::<BatchListResponse>(
                r#"{"object":"future","data":null,"has_more":null}"#
            )
            .is_err()
        );
    }

    #[test]
    fn list_object_marker_is_forward_compatible_only_at_the_wire_field() {
        let future: BatchListResponse = serde_json::from_str(
            r#"{"object":"future_list","first_id":"batch-1","has_more":false}"#,
        )
        .unwrap();
        assert!(future.object.is_none());
        assert_eq!(future.first_id.as_deref(), Some("batch-1"));
        assert_eq!(future.has_more, Some(false));

        let known: BatchListResponse = serde_json::from_str(r#"{"object":"list"}"#).unwrap();
        assert!(matches!(known.object, Some(BatchListObject::List)));
        let missing: BatchListResponse = serde_json::from_str(r#"{"last_id":"batch-2"}"#).unwrap();
        assert!(missing.object.is_none());
        assert_eq!(missing.last_id.as_deref(), Some("batch-2"));
        let null: BatchListResponse = serde_json::from_str(r#"{"object":null,"data":[]}"#).unwrap();
        assert!(null.object.is_none());
        assert!(null.data.as_ref().is_some_and(Vec::is_empty));

        for marker in ["1", "true", "[]", "{}", r#"{"future-list":null}"#] {
            let text = format!(r#"{{"object":{marker},"has_more":false}}"#);
            assert!(serde_json::from_str::<BatchListResponse>(&text).is_err());
        }
        assert!(serde_json::from_str::<BatchListObject>(r#""future_list""#).is_err());

        let encoded = serde_json::to_value(BatchListResponse {
            object: Some(BatchListObject::List),
            data: None,
            first_id: None,
            last_id: None,
            has_more: None,
        })
        .unwrap();
        assert_eq!(encoded["object"], "list");
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

    #[test]
    fn validated_pagination_maps_without_changing_legacy_limit_behavior() {
        let pagination = CursorPagination::new()
            .try_with_after("batch-cursor")
            .unwrap()
            .try_with_limit(u32::MAX)
            .unwrap();
        let query = BatchListQuery::new()
            .try_with_pagination(pagination)
            .unwrap();
        assert_eq!(query.after.as_deref(), Some("batch-cursor"));
        assert_eq!(query.limit, Some(u32::MAX));

        assert!(BatchListQuery::new().with_limit(0).validate().is_ok());
    }

    #[test]
    fn next_after_requires_explicit_more_data_and_a_non_blank_last_id() {
        fn response(has_more: Option<bool>, last_id: Option<&str>) -> BatchListResponse {
            BatchListResponse {
                object: None,
                data: None,
                first_id: None,
                last_id: last_id.map(str::to_owned),
                has_more,
            }
        }

        assert_eq!(
            response(Some(true), Some("batch-2")).next_after(),
            Some("batch-2")
        );
        assert_eq!(response(Some(false), Some("batch-2")).next_after(), None);
        assert_eq!(response(None, Some("batch-2")).next_after(), None);

        for last_id in [None, Some(""), Some(" \t\u{2003}")] {
            assert_eq!(response(Some(true), last_id).next_after(), None);
        }

        assert_eq!(
            response(Some(true), Some(" batch-2 ")).next_after(),
            Some(" batch-2 ")
        );
    }
}
