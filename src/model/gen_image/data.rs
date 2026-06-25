use std::sync::Arc;

use serde::Serialize;
use validator::Validate;

use super::{
    super::traits::*,
    image_request::{ImageGenBody, ImageQuality, ImageSize},
};
use crate::client::{
    endpoints::{ApiBase, EndpointConfig, paths},
    http::{HttpClient, HttpClientConfig, parse_typed_response},
};

/// Image generation request structure
/// Provides a typed builder around the image generation API body
pub struct ImageGenRequest<N>
where
    N: ModelName + ImageGen + Serialize,
{
    /// API key for authentication
    pub key: String,
    url: String,
    endpoint_config: EndpointConfig,
    api_base: ApiBase,
    http_config: Arc<HttpClientConfig>,
    /// Request body
    body: ImageGenBody<N>,
}

impl<N> ImageGenRequest<N>
where
    N: ModelName + ImageGen + Serialize,
{
    /// Create a new image generation request for the given model and API key
    pub fn new(model: N, key: impl Into<String>) -> Self {
        let body = ImageGenBody {
            model,
            prompt: None,
            quality: None,
            size: None,
            watermark_enabled: None,
            user_id: None,
        };
        let endpoint_config = EndpointConfig::default();
        let api_base = ApiBase::PaasV4;
        let url = endpoint_config.url(&api_base, paths::IMAGES_GENERATIONS);
        Self {
            key: key.into(),
            url,
            endpoint_config,
            api_base,
            http_config: Arc::new(HttpClientConfig::default()),
            body,
        }
    }

    fn rebuild_url(&mut self) {
        self.url = self
            .endpoint_config
            .url(&self.api_base, paths::IMAGES_GENERATIONS);
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

    /// Mutable access to inner body (for advanced customizations)
    pub fn body_mut(&mut self) -> &mut ImageGenBody<N> {
        &mut self.body
    }

    /// Set prompt text
    pub fn with_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.body.prompt = Some(prompt.into());
        self
    }

    /// Set image quality
    pub fn with_quality(mut self, quality: ImageQuality) -> Self {
        self.body.quality = Some(quality);
        self
    }

    /// Set image size
    pub fn with_size(mut self, size: ImageSize) -> Self {
        self.body.size = Some(size);
        self
    }

    /// Enable/disable watermark
    pub fn with_watermark_enabled(mut self, watermark_enabled: bool) -> Self {
        self.body.watermark_enabled = Some(watermark_enabled);
        self
    }

    /// Set user id
    pub fn with_user_id(mut self, user_id: impl Into<String>) -> Self {
        self.body.user_id = Some(user_id.into());
        self
    }

    /// Validate body constraints: required prompt and (when set) a valid
    /// custom image size.
    pub fn validate(&self) -> crate::ZaiResult<()> {
        // Body-level field validations
        self.body
            .validate()
            .map_err(|e| crate::client::error::ZaiError::ApiError {
                code: crate::client::error::codes::SDK_VALIDATION,
                message: format!("Validation error: {:?}", e),
            })?;
        // Require prompt
        if self
            .body
            .prompt
            .as_deref()
            .map(|s| s.trim().is_empty())
            .unwrap_or(true)
        {
            return Err(crate::client::error::ZaiError::ApiError {
                code: crate::client::error::codes::SDK_VALIDATION,
                message: "prompt is required".to_string(),
            });
        }
        // Validate custom size when present
        if let Some(size) = &self.body.size
            && let super::image_request::ImageSize::Custom { .. } = size
            && !size.is_valid()
        {
            return Err(crate::client::error::ZaiError::ApiError {
                        code: crate::client::error::codes::SDK_VALIDATION,
                        message: "invalid custom image size: must be 512..=2048, divisible by 16, and <= 2^21 pixels".to_string(),
                    });
        }
        Ok(())
    }

    /// Submit the request and parse the typed image-generation response.
    pub async fn send(&self) -> crate::ZaiResult<super::image_response::ImageResponse> {
        self.validate()?;
        let resp = self.post().await?;
        let parsed = parse_typed_response::<super::image_response::ImageResponse>(resp).await?;
        Ok(parsed)
    }
}

impl<N> HttpClient for ImageGenRequest<N>
where
    N: ModelName + ImageGen + Serialize,
{
    type Body = ImageGenBody<N>;
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

    /// Serialized request body.
    fn body(&self) -> &Self::Body {
        &self.body
    }

    /// HTTP client configuration (timeouts, retries, …).
    fn http_config(&self) -> Arc<HttpClientConfig> {
        Arc::clone(&self.http_config)
    }
}
