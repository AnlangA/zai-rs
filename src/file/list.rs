use super::request::FileListQuery;
use crate::{ZaiResult, client::ZaiClient};

/// Files list request (GET /paas/v4/files)
///
/// Builds query parameters from `FileListQuery` and performs an authenticated
/// GET.
pub struct FileListRequest {
    query: FileListQuery,
}

impl FileListRequest {
    /// Create a new file-list request (empty query).
    pub fn new() -> Self {
        Self {
            query: FileListQuery::new(),
        }
    }

    /// Replace the query parameters.
    pub fn with_query(mut self, q: FileListQuery) -> Self {
        self.query = q;
        self
    }

    /// Send the configured query and parse the typed response.
    ///
    /// This method does not run [`validator::Validate`]; use
    /// [`Self::send_with_query_via`] when the query needs local range validation.
    pub async fn send_via(
        &self,
        client: &ZaiClient,
    ) -> ZaiResult<super::response::FileListResponse> {
        let mut params: Vec<(&str, String)> = Vec::new();
        if let Some(after) = self.query.after.as_ref() {
            params.push(("after", after.clone()));
        }
        if let Some(purpose) = self.query.purpose.as_ref() {
            params.push(("purpose", purpose.as_str().to_string()));
        }
        if let Some(order) = self.query.order.as_ref() {
            params.push(("order", order.as_str().to_string()));
        }
        if let Some(limit) = self.query.limit.as_ref() {
            params.push(("limit", limit.to_string()));
        }
        let borrowed: Vec<(&str, &str)> = params.iter().map(|(k, v)| (*k, v.as_str())).collect();
        let route = crate::client::routes::FILES_LIST;
        let url = client
            .endpoints()
            .resolve_route_with_query(route, &[], &borrowed)?;
        client
            .send_empty::<super::response::FileListResponse>(route.method(), url)
            .await
    }

    /// Validate query and send in one call.
    pub async fn send_with_query_via(
        mut self,
        client: &ZaiClient,
        q: &FileListQuery,
    ) -> ZaiResult<super::response::FileListResponse> {
        use validator::Validate;
        q.validate()?;
        self.query = q.clone();
        self.send_via(client).await
    }
}

impl Default for FileListRequest {
    fn default() -> Self {
        Self::new()
    }
}
