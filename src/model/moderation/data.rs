//! # Content Moderation API
//!
//! This module provides the content moderation client for analyzing text,
//! image, audio, and video content for safety risks.

use std::sync::Arc;

use super::models::*;
use crate::client::{
    endpoints::{ApiBase, EndpointConfig, paths},
    http::{HttpClient, HttpClientConfig, parse_typed_response},
};

/// Content moderation client.
///
/// This client provides functionality to moderate content for safety risks,
/// supporting text, image, audio, and video formats.
///
/// ## Examples
///
/// ```rust,ignore
/// let api_key = "your-api-key".to_string();
/// let moderation = Moderation::new_text("审核内容安全样例字符串。", api_key);
/// let result = moderation.send().await?;
/// ```
pub struct Moderation {
    /// API key for authentication
    pub key: String,
    url: String,
    endpoint_config: EndpointConfig,
    api_base: ApiBase,
    http_config: Arc<HttpClientConfig>,
    /// Moderation request body
    body: ModerationRequest,
}

impl Moderation {
    /// Creates a new moderation request with text content.
    ///
    /// ## Arguments
    ///
    /// * `text` - The text content to moderate (max 2000 characters)
    /// * `key` - API key for authentication
    ///
    /// ## Returns
    ///
    /// A new `Moderation` instance configured for text moderation.
    pub fn new_text(text: impl Into<String>, key: String) -> Self {
        let endpoint_config = EndpointConfig::default();
        let api_base = ApiBase::PaasV4;
        let url = endpoint_config.url(&api_base, paths::MODERATIONS);
        let body = ModerationRequest::new_text(text);
        Self {
            body,
            key,
            url,
            endpoint_config,
            api_base,
            http_config: Arc::new(HttpClientConfig::default()),
        }
    }

    /// Creates a new moderation request with multimedia content.
    ///
    /// ## Arguments
    ///
    /// * `content_type` - The type of multimedia content (image, audio, video)
    /// * `url` - URL to the multimedia content
    /// * `key` - API key for authentication
    ///
    /// ## Returns
    ///
    /// A new `Moderation` instance configured for multimedia moderation.
    pub fn new_multimedia(content_type: MediaType, url: impl Into<String>, key: String) -> Self {
        let endpoint_config = EndpointConfig::default();
        let api_base = ApiBase::PaasV4;
        let endpoint_url = endpoint_config.url(&api_base, paths::MODERATIONS);
        let body = ModerationRequest::new_multimedia(content_type, url);
        Self {
            body,
            key,
            url: endpoint_url,
            endpoint_config,
            api_base,
            http_config: Arc::new(HttpClientConfig::default()),
        }
    }

    fn rebuild_url(&mut self) {
        self.url = self.endpoint_config.url(&self.api_base, paths::MODERATIONS);
    }

    pub fn with_base_url(mut self, base: impl Into<String>) -> Self {
        self.api_base = ApiBase::Custom(base.into());
        self.rebuild_url();
        self
    }

    pub fn with_endpoint_config(mut self, endpoint_config: EndpointConfig) -> Self {
        self.endpoint_config = endpoint_config;
        self.rebuild_url();
        self
    }

    pub fn with_http_config(mut self, config: HttpClientConfig) -> Self {
        self.http_config = Arc::new(config);
        self
    }

    /// Gets mutable access to the request body for further customization.
    pub fn body_mut(&mut self) -> &mut ModerationRequest {
        &mut self.body
    }

    pub fn validate(&self) -> crate::ZaiResult<()> {
        self.body
            .validate()
            .map_err(crate::client::error::ZaiError::from)?;
        Ok(())
    }

    /// Sends the moderation request and returns the structured response.
    ///
    /// This method automatically validates the request before sending.
    ///
    /// ## Returns
    ///
    /// A `ModerationResponse` containing the moderation results and usage
    /// statistics.
    pub async fn send(&self) -> crate::ZaiResult<ModerationResponse> {
        self.validate()?;

        let resp: reqwest::Response = self.post().await?;

        let parsed = parse_typed_response::<ModerationResponse>(resp).await?;

        Ok(parsed)
    }
}

impl HttpClient for Moderation {
    type Body = ModerationRequest;
    type ApiUrl = String;
    type ApiKey = String;

    /// Returns the Zhipu AI moderation API endpoint URL.
    fn api_url(&self) -> &Self::ApiUrl {
        &self.url
    }

    fn api_key(&self) -> &Self::ApiKey {
        &self.key
    }

    fn body(&self) -> &Self::Body {
        &self.body
    }

    fn http_config(&self) -> Arc<HttpClientConfig> {
        Arc::clone(&self.http_config)
    }
}
