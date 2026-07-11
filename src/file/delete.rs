use crate::client::ZaiClient;

/// File delete request (DELETE /paas/v4/files/{file_id})
pub struct FileDeleteRequest {
    file_id: String,
}

impl FileDeleteRequest {
    /// Create a new delete request for the given file id.
    pub fn new(file_id: impl Into<String>) -> Self {
        Self {
            file_id: file_id.into(),
        }
    }

    /// Send delete request via a [`ZaiClient`] and parse typed response.
    pub async fn send_via(
        &self,
        client: &ZaiClient,
    ) -> crate::ZaiResult<super::response::FileDeleteResponse> {
        let route = crate::client::routes::FILES_DELETE;
        let url = client.endpoints().resolve_route(route, &[&self.file_id])?;
        client
            .send_empty::<super::response::FileDeleteResponse>(route.method(), url)
            .await
    }
}
