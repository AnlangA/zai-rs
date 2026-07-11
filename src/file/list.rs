use std::sync::Arc;

use super::request::FileListQuery;
use crate::{
    ZaiResult,
    client::{
        http::{HttpClientConfig, parse_typed_response, send_empty_request},
        {ApiFamily, ZaiClient},
    },
};

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

    /// Send request and parse typed response.
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
        let url =
            client
                .endpoints()
                .resolve_with_query(ApiFamily::PaasV4, &["files"], &borrowed)?;
        let config = transport_config_from_client(client);
        let resp = send_empty_request(
            reqwest::Method::GET,
            url,
            client.secret().expose(),
            Arc::new(config),
        )
        .await?;
        parse_typed_response::<super::response::FileListResponse>(resp).await
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
