use super::request::{FileListPurpose, FileListQuery};
use crate::{ZaiResult, client::ZaiClient};

/// Files list request (GET /paas/v4/files)
///
/// Builds query parameters from `FileListQuery` and performs an authenticated
/// GET.
pub struct FileListRequest {
    query: FileListQuery,
}

impl FileListRequest {
    /// Create a new file-list request with its required purpose filter.
    pub fn new(purpose: FileListPurpose) -> Self {
        Self {
            query: FileListQuery::new(purpose),
        }
    }

    /// Replace the query parameters.
    pub fn with_query(mut self, q: FileListQuery) -> Self {
        self.query = q;
        self
    }

    /// Validate the configured query, send it, and parse the typed response.
    pub async fn send_via(
        &self,
        client: &ZaiClient,
    ) -> ZaiResult<super::response::FileListResponse> {
        use validator::Validate;

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
        params.push(("purpose", self.query.purpose.as_str().to_string()));
        if let Some(order) = self.query.order.as_ref() {
            params.push(("order", order.as_str().to_string()));
        }
        if let Some(limit) = self.query.limit.as_ref() {
            params.push(("limit", limit.to_string()));
        }
        let borrowed: Vec<(&str, &str)> = params.iter().map(|(k, v)| (*k, v.as_str())).collect();
        let route = crate::client::routes::FILES_LIST;
        client
            .operation(route)
            .with_query(borrowed)
            .send_empty::<super::response::FileListResponse>()
            .await
    }
}
