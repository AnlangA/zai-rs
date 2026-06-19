use std::sync::Arc;

use crate::client::{
    endpoints::{ApiBase, EndpointConfig, join_url, paths},
    http::{HttpClient, HttpClientConfig, parse_typed_response},
};

/// Knowledge delete request (DELETE /llm-application/open/knowledge/{id})
pub struct KnowledgeDeleteRequest {
    /// Bearer API key
    pub key: String,
    url: String,
    endpoint_config: EndpointConfig,
    api_base: ApiBase,
    id: String,
    http_config: Arc<HttpClientConfig>,
    _body: (),
}

impl KnowledgeDeleteRequest {
    /// Build a delete request with target id
    pub fn new(key: String, id: impl AsRef<str>) -> Self {
        let id = id.as_ref().to_string();
        let endpoint_config = EndpointConfig::default();
        let api_base = ApiBase::LlmApplication;
        let url = endpoint_config.url(&api_base, &join_url(paths::KNOWLEDGE, &id));
        Self {
            key,
            url,
            endpoint_config,
            api_base,
            id,
            http_config: Arc::new(HttpClientConfig::default()),
            _body: (),
        }
    }

    fn rebuild_url(&mut self) {
        self.url = self
            .endpoint_config
            .url(&self.api_base, &join_url(paths::KNOWLEDGE, &self.id));
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

    /// Issue the underlying DELETE request.
    pub fn delete(
        &self,
    ) -> impl std::future::Future<Output = crate::ZaiResult<reqwest::Response>> + Send {
        HttpClient::delete(self)
    }

    /// Send delete request and parse typed response
    pub async fn send(&self) -> crate::ZaiResult<KnowledgeDeleteResponse> {
        let resp = self.delete().await?;
        parse_typed_response::<KnowledgeDeleteResponse>(resp).await
    }
}

impl HttpClient for KnowledgeDeleteRequest {
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
    /// Empty body placeholder (DELETE request).
    fn body(&self) -> &Self::Body {
        &self._body
    }

    /// HTTP client configuration (timeouts, retries, …).
    fn http_config(&self) -> Arc<HttpClientConfig> {
        self.http_config.clone()
    }
}

/// Delete response envelope without data
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, validator::Validate)]
pub struct KnowledgeDeleteResponse {
    /// Business status code.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<i64>,
    /// Human-readable message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// Server timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<u64>,
}
