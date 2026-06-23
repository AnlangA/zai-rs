//! Central SDK configuration — the single source of truth for credentials,
//! endpoint base URLs, and HTTP transport settings.
//!
//! Borrows the configuration ergonomics popularized by `async-openai`
//! (`OpenAIConfig { api_key, base_url }` + `Client::with_config` / `from_env`):
//! one struct holds everything, a builder constructs it, and
//! [`ZaiConfig::from_env`] reads `ZHIPU_API_KEY` directly.
//!
//! `ZaiConfig` intentionally **nests** [`HttpClientConfig`] (rather than
//! flattening it) so the existing per-request `with_http_config` builders and
//! the `HttpClient::http_config() -> Arc<HttpClientConfig>` contract remain
//! untouched — a focused, low-blast-radius addition.

use crate::{
    ZaiError, ZaiResult,
    client::{
        endpoints::{ApiBase, EndpointConfig},
        http::HttpClientConfig,
    },
};

/// Central SDK configuration.
#[derive(Debug, Clone, Default)]
pub struct ZaiConfig {
    /// Zhipu API key in `<id>.<secret>` form.
    pub api_key: String,
    /// Configurable base URLs for each API family.
    pub endpoints: EndpointConfig,
    /// HTTP transport settings (timeouts, retries, masking, …).
    pub http: HttpClientConfig,
    /// Optional pre-built `reqwest::Client` (e.g. for a custom connector pool).
    pub reqwest: Option<reqwest::Client>,
}

impl ZaiConfig {
    /// Start a builder.
    pub fn builder() -> ZaiConfigBuilder {
        ZaiConfigBuilder {
            config: ZaiConfig::default(),
        }
    }

    /// Build a config from an API key using the official default endpoints and
    /// HTTP settings.
    pub fn new(api_key: impl Into<String>) -> ZaiResult<Self> {
        Self::builder().api_key(api_key).build()
    }

    /// Read `ZHIPU_API_KEY` from the environment and use default endpoints/HTTP
    /// settings.
    pub fn from_env() -> ZaiResult<Self> {
        let api_key = std::env::var("ZHIPU_API_KEY").map_err(|_| ZaiError::AuthError {
            code: 1001,
            message: "ZHIPU_API_KEY environment variable not set".to_string(),
        })?;
        Ok(Self {
            api_key,
            ..ZaiConfig::default()
        })
    }

    /// Resolve the realtime WebSocket URL from this config's endpoints.
    pub fn realtime_url(&self) -> String {
        self.endpoints.url(&ApiBase::Realtime, "")
    }

    /// Resolve a PAAS v4 REST URL for the given path.
    pub fn paas_v4_url(&self, path: &str) -> String {
        self.endpoints.url(&ApiBase::PaasV4, path)
    }

    /// Resolve a Coding Plan PAAS v4 URL for the given path.
    pub fn coding_paas_v4_url(&self, path: &str) -> String {
        self.endpoints.url(&ApiBase::CodingPaasV4, path)
    }

    /// Resolve a knowledge-base (LLM application) URL for the given path.
    pub fn llm_application_url(&self, path: &str) -> String {
        self.endpoints.url(&ApiBase::LlmApplication, path)
    }

    /// Resolve a monitor / usage-statistics URL for the given path
    /// (e.g. Coding Plan quota query).
    pub fn monitor_url(&self, path: &str) -> String {
        self.endpoints.url(&ApiBase::Monitor, path)
    }
}

/// Builder for [`ZaiConfig`].
#[derive(Debug, Clone, Default)]
pub struct ZaiConfigBuilder {
    config: ZaiConfig,
}

impl ZaiConfigBuilder {
    /// Set the API key (required).
    pub fn api_key(mut self, api_key: impl Into<String>) -> Self {
        self.config.api_key = api_key.into();
        self
    }

    /// Replace the entire endpoint config.
    pub fn endpoint_config(mut self, endpoints: EndpointConfig) -> Self {
        self.config.endpoints = endpoints;
        self
    }

    /// Override the general PAAS v4 base URL.
    pub fn paas_v4_base(mut self, base: impl Into<String>) -> Self {
        self.config.endpoints = self.config.endpoints.with_paas_v4_base(base);
        self
    }

    /// Override the Coding Plan PAAS v4 base URL.
    pub fn coding_paas_v4_base(mut self, base: impl Into<String>) -> Self {
        self.config.endpoints = self.config.endpoints.with_coding_paas_v4_base(base);
        self
    }

    /// Override the knowledge-base / LLM application base URL.
    pub fn llm_application_base(mut self, base: impl Into<String>) -> Self {
        self.config.endpoints = self.config.endpoints.with_llm_application_base(base);
        self
    }

    /// Override the realtime base URL.
    pub fn realtime_base(mut self, base: impl Into<String>) -> Self {
        self.config.endpoints = self.config.endpoints.with_realtime_base(base);
        self
    }

    /// Override the monitor / usage-statistics base URL.
    pub fn monitor_base(mut self, base: impl Into<String>) -> Self {
        self.config.endpoints = self.config.endpoints.with_monitor_base(base);
        self
    }

    /// Replace the HTTP transport config.
    pub fn http_config(mut self, http: HttpClientConfig) -> Self {
        self.config.http = http;
        self
    }

    /// Provide a pre-built `reqwest::Client`.
    pub fn reqwest_client(mut self, client: reqwest::Client) -> Self {
        self.config.reqwest = Some(client);
        self
    }

    /// Finalize. Fails if `api_key` was never set.
    pub fn build(self) -> ZaiResult<ZaiConfig> {
        if self.config.api_key.is_empty() {
            return Err(ZaiError::ApiError {
                code: 1200,
                message: "ZaiConfig requires an api_key".to_string(),
            });
        }
        Ok(self.config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::endpoints::REALTIME_BASE;

    #[test]
    fn builder_requires_api_key() {
        assert!(ZaiConfig::builder().build().is_err());
        let cfg = ZaiConfig::new("abcdefghij.0123456789abcdef").unwrap();
        assert_eq!(cfg.api_key, "abcdefghij.0123456789abcdef");
    }

    #[test]
    fn realtime_url_uses_official_endpoint() {
        let cfg = ZaiConfig::builder()
            .api_key("abcdefghij.0123456789abcdef")
            .build()
            .unwrap();
        assert_eq!(cfg.realtime_url(), REALTIME_BASE);
    }

    #[test]
    fn custom_realtime_base_overrides() {
        let cfg = ZaiConfig::builder()
            .api_key("abcdefghij.0123456789abcdef")
            .realtime_base("wss://custom.example.com/realtime")
            .build()
            .unwrap();
        assert_eq!(cfg.realtime_url(), "wss://custom.example.com/realtime");
    }

    #[test]
    fn builder_overrides_all_rest_base_families() {
        let cfg = ZaiConfig::builder()
            .api_key("abcdefghij.0123456789abcdef")
            .paas_v4_base("https://proxy.example.com/api/paas/v4")
            .coding_paas_v4_base("https://proxy.example.com/api/coding/paas/v4")
            .llm_application_base("https://proxy.example.com/api/llm-application/open")
            .monitor_base("https://proxy.example.com/api/monitor")
            .build()
            .unwrap();

        assert_eq!(
            cfg.paas_v4_url("chat/completions"),
            "https://proxy.example.com/api/paas/v4/chat/completions"
        );
        assert_eq!(
            cfg.coding_paas_v4_url("chat/completions"),
            "https://proxy.example.com/api/coding/paas/v4/chat/completions"
        );
        assert_eq!(
            cfg.llm_application_url("knowledge"),
            "https://proxy.example.com/api/llm-application/open/knowledge"
        );
        assert_eq!(
            cfg.monitor_url("usage/quota/limit"),
            "https://proxy.example.com/api/monitor/usage/quota/limit"
        );
    }
}
