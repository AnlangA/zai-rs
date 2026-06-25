use std::sync::Arc;

use super::types::BatchItem;
use crate::{
    ZaiResult,
    client::{
        endpoints::{ApiBase, EndpointConfig, join_url, paths},
        http::{HttpClient, HttpClientConfig, parse_typed_response},
    },
};

/// Retrieve a batch task by ID (GET /paas/v4/batches/{batch_id})
pub struct BatchesRetrieveRequest {
    /// Bearer API key
    pub key: String,
    /// Full URL with path parameter bound
    url: String,
    endpoint_config: EndpointConfig,
    api_base: ApiBase,
    batch_id: String,
    http_config: Arc<HttpClientConfig>,
    /// No body for GET
    _body: (),
}

impl BatchesRetrieveRequest {
    /// Create a new retrieve request with required path parameter `batch_id`.
    pub fn new(key: impl Into<String>, batch_id: impl Into<String>) -> Self {
        // Batch IDs are expected to be safe; if special chars appear, consider
        // encoding.
        let batch_id = batch_id.into();
        let endpoint_config = EndpointConfig::default();
        let api_base = ApiBase::PaasV4;
        let url = endpoint_config.url(&api_base, &join_url(paths::BATCHES, &batch_id));
        Self {
            key: key.into(),
            url,
            endpoint_config,
            api_base,
            batch_id,
            http_config: Arc::new(HttpClientConfig::default()),
            _body: (),
        }
    }

    fn rebuild_url(&mut self) {
        self.url = self
            .endpoint_config
            .url(&self.api_base, &join_url(paths::BATCHES, &self.batch_id));
    }

    /// Override the base URL (uses [`ApiBase::Custom`]).
    pub fn with_base_url(mut self, base: impl Into<String>) -> Self {
        self.api_base = ApiBase::Custom(base.into());
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

    /// Send request and parse typed response as a single BatchItem
    pub async fn send(&self) -> ZaiResult<BatchesRetrieveResponse> {
        let resp: reqwest::Response = self.get().await?;
        let parsed = parse_typed_response::<BatchesRetrieveResponse>(resp).await?;
        Ok(parsed)
    }
}

impl HttpClient for BatchesRetrieveRequest {
    type Body = ();
    type ApiUrl = String;
    type ApiKey = String;

    /// Resolved target URL for the request.
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
        Arc::clone(&self.http_config)
    }
}

/// Response type: a single Batch object
pub type BatchesRetrieveResponse = BatchItem;
