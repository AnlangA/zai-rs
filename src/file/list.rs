use std::sync::Arc;

use super::request::FileListQuery;
use crate::{
    ZaiResult,
    client::{
        endpoints::{ApiBase, EndpointConfig, build_query, paths},
        http::{HttpClient, HttpClientConfig, parse_typed_response},
    },
};

/// Files list request (GET /paas/v4/files)
///
/// Builds query parameters from `FileListQuery` and performs an authenticated
/// GET.
pub struct FileListRequest {
    /// Zhipu AI API key used for `Authorization: Bearer …`.
    pub key: String,
    url: String,
    endpoint_config: EndpointConfig,
    api_base: ApiBase,
    query: FileListQuery,
    _body: (),
    http_config: Arc<HttpClientConfig>,
}

impl FileListRequest {
    /// Create a new file-list request (empty query).
    pub fn new(key: impl Into<String>) -> Self {
        let endpoint_config = EndpointConfig::default();
        let api_base = ApiBase::PaasV4;
        let url = endpoint_config.url(&api_base, paths::FILES);
        Self {
            key: key.into(),
            url,
            endpoint_config,
            api_base,
            query: FileListQuery::new(),
            _body: (),
            http_config: Arc::new(HttpClientConfig::default()),
        }
    }

    fn rebuild_url(&mut self) {
        let endpoint = self.endpoint_config.url(&self.api_base, paths::FILES);
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
        self.url = build_query(&endpoint, params);
    }

    /// Override the base URL (uses [`ApiBase::Custom`]).
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.api_base = ApiBase::Custom(base_url.into());
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

    /// Replace the query parameters.
    pub fn with_query(mut self, q: FileListQuery) -> Self {
        self.query = q;
        self.rebuild_url();
        self
    }
    /// Send request and parse typed response.
    pub async fn send(&self) -> ZaiResult<super::response::FileListResponse> {
        let resp = self.get().await?;
        parse_typed_response::<super::response::FileListResponse>(resp).await
    }

    /// Validate query, rebuild URL and send in one call.
    pub async fn send_with_query(
        mut self,
        q: &super::request::FileListQuery,
    ) -> ZaiResult<super::response::FileListResponse> {
        use validator::Validate;
        q.validate()?;
        self.query = q.clone();
        self.rebuild_url();
        self.send().await
    }
}

impl HttpClient for FileListRequest {
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
        self.http_config.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::http::HttpClient;

    #[test]
    fn query_survives_base_rebuild() {
        let request = FileListRequest::new("test.12345678901234567890".to_string())
            .with_query(
                FileListQuery::new()
                    .with_purpose(crate::file::request::FilePurpose::Batch)
                    .with_limit(2),
            )
            .with_base_url("http://127.0.0.1:12345/api/paas/v4");

        assert_eq!(
            request.api_url(),
            "http://127.0.0.1:12345/api/paas/v4/files?purpose=batch&limit=2"
        );
    }

    #[test]
    fn query_survives_endpoint_config_rebuild() {
        let endpoint_config =
            EndpointConfig::default().with_paas_v4_base("http://127.0.0.1:12345/api/paas/v4");
        let request = FileListRequest::new("test.12345678901234567890".to_string())
            .with_query(FileListQuery::new().with_after("cursor").with_limit(10))
            .with_endpoint_config(endpoint_config);

        assert_eq!(
            request.api_url(),
            "http://127.0.0.1:12345/api/paas/v4/files?after=cursor&limit=10"
        );
    }
}
