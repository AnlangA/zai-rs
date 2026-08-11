use super::request::{FileListPurpose, FileListQuery};
use crate::{ZaiResult, client::ZaiClient, pagination::CursorPagination};

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

    /// Replace the request's cursor and limit with validated pagination.
    pub fn try_with_pagination(mut self, pagination: CursorPagination) -> ZaiResult<Self> {
        self.query = self.query.try_with_pagination(pagination)?;
        Ok(self)
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
        let route = crate::client::routes::FILES_LIST;
        client
            .operation(route)
            .with_query(&self.query)?
            .send_empty::<super::response::FileListResponse>()
            .await
    }
}
