use std::sync::Arc;

use serde::{Deserialize, Serialize};

use super::types::BatchItem;
use crate::{
    ZaiResult,
    client::{
        endpoints::{ApiBase, EndpointConfig, join_url, paths},
        http::{HttpClient, HttpClientConfig, parse_typed_response},
    },
};

/// Empty body for cancel API (serializes to `{}`)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CancelBatchBody {}

/// Cancel a running batch (POST /paas/v4/batches/{batch_id}/cancel)
pub struct CancelBatchRequest {
    /// Bearer API key
    pub key: String,
    /// Full URL including path parameter
    url: String,
    endpoint_config: EndpointConfig,
    api_base: ApiBase,
    batch_id: String,
    http_config: Arc<HttpClientConfig>,
    /// Empty JSON body
    body: CancelBatchBody,
}

impl CancelBatchRequest {
    /// Create a new cancel request for the given batch_id
    pub fn new(key: impl Into<String>, batch_id: impl Into<String>) -> Self {
        let batch_id = batch_id.into();
        let endpoint_config = EndpointConfig::default();
        let api_base = ApiBase::PaasV4;
        let path = join_url(&join_url(paths::BATCHES, &batch_id), "cancel");
        let url = endpoint_config.url(&api_base, &path);
        Self {
            key: key.into(),
            url,
            endpoint_config,
            api_base,
            batch_id,
            http_config: Arc::new(HttpClientConfig::default()),
            body: CancelBatchBody::default(),
        }
    }

    fn rebuild_url(&mut self) {
        let path = join_url(&join_url(paths::BATCHES, &self.batch_id), "cancel");
        self.url = self.endpoint_config.url(&self.api_base, &path);
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

    /// Send the request and parse typed response
    pub async fn send(&self) -> ZaiResult<CancelBatchResponse> {
        let resp: reqwest::Response = self.post().await?;
        let parsed = parse_typed_response::<CancelBatchResponse>(resp).await?;
        Ok(parsed)
    }
}

impl HttpClient for CancelBatchRequest {
    type Body = CancelBatchBody;
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
    /// Empty JSON body (`{}`).
    fn body(&self) -> &Self::Body {
        &self.body
    }

    /// HTTP client configuration (timeouts, retries, …).
    fn http_config(&self) -> Arc<HttpClientConfig> {
        Arc::clone(&self.http_config)
    }
}

/// Response type: a single Batch object
pub type CancelBatchResponse = BatchItem;
