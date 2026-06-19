use std::sync::Arc;

use serde::Serialize;
use validator::Validate;

use super::{
    super::traits::*,
    video_request::{Fps, ImageUrl, VideoBody, VideoDuration, VideoQuality, VideoSize},
};
use crate::client::{
    endpoints::{ApiBase, EndpointConfig, paths},
    http::{HttpClient, HttpClientConfig},
};

/// Video generation request structure
/// Handles HTTP requests for video generation API
pub struct VideoGenRequest<N>
where
    N: ModelName + VideoGen + Serialize,
{
    /// API key for authentication
    pub key: String,
    url: String,
    endpoint_config: EndpointConfig,
    api_base: ApiBase,
    http_config: Arc<HttpClientConfig>,
    /// Request Body
    body: VideoBody<N>,
}

impl<N> VideoGenRequest<N>
where
    N: ModelName + VideoGen + Serialize,
{
    /// Create a new video generation request
    ///
    /// # Arguments
    /// * `model` - Video generation model implementing VideoGen trait
    /// * `body` - Video generation parameters and configuration
    /// * `key` - API key for authentication
    pub fn new(model: N, key: String) -> Self {
        let body = VideoBody::new(model);
        let endpoint_config = EndpointConfig::default();
        let api_base = ApiBase::PaasV4;
        let url = endpoint_config.url(&api_base, paths::VIDEOS_GENERATIONS);
        Self {
            key,
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
            .url(&self.api_base, paths::VIDEOS_GENERATIONS);
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

    /// Set the prompt for video generation
    pub fn with_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.body = self.body.with_prompt(prompt);
        self
    }

    /// Set the quality mode (speed or quality)
    pub fn with_quality(mut self, quality: VideoQuality) -> Self {
        self.body = self.body.with_quality(quality);
        self
    }

    /// Enable/disable audio generation
    pub fn with_audio(mut self, with_audio: bool) -> Self {
        self.body = self.body.with_audio(with_audio);
        self
    }

    /// Enable/disable watermark
    pub fn with_watermark_enabled(mut self, watermark_enabled: bool) -> Self {
        self.body = self.body.with_watermark_enabled(watermark_enabled);
        self
    }

    /// Set image URL(s) for video generation
    pub fn with_image_url(mut self, image_url: ImageUrl) -> Self {
        self.body = self.body.with_image_url(image_url);
        self
    }

    /// Set video resolution size
    pub fn with_size(mut self, size: VideoSize) -> Self {
        self.body = self.body.with_size(size);
        self
    }

    /// Set video frame rate (30 or 60 FPS)
    pub fn with_fps(mut self, fps: Fps) -> Self {
        self.body = self.body.with_fps(fps);
        self
    }

    /// Set video duration (5 or 10 seconds)
    pub fn with_duration(mut self, duration: VideoDuration) -> Self {
        self.body = self.body.with_duration(duration);
        self
    }

    /// Set custom request ID
    pub fn with_request_id(mut self, request_id: String) -> Self {
        self.body = self.body.with_request_id(request_id);
        self
    }

    /// Set user ID for policy enforcement
    pub fn with_user_id(mut self, user_id: String) -> Self {
        self.body = self.body.with_user_id(user_id);
        self
    }
}

impl<N> VideoGenRequest<N>
where
    N: ModelName + VideoGen + Serialize,
{
    /// Validate request parameters for video generation
    pub fn validate(&self) -> crate::ZaiResult<()> {
        self.body
            .validate()
            .map_err(crate::client::error::ZaiError::from)?;
        Ok(())
    }
}

impl<N> HttpClient for VideoGenRequest<N>
where
    N: ModelName + VideoGen + Serialize,
{
    type Body = VideoBody<N>;
    /// API URL type
    type ApiUrl = String;
    /// API key type
    type ApiKey = String;

    /// Get the API endpoint URL
    fn api_url(&self) -> &Self::ApiUrl {
        &self.url
    }

    /// Get the API key for authentication
    fn api_key(&self) -> &Self::ApiKey {
        &self.key
    }

    /// Get the request body containing video generation parameters
    fn body(&self) -> &Self::Body {
        &self.body
    }

    /// HTTP client configuration (timeouts, retries, …).
    fn http_config(&self) -> Arc<HttpClientConfig> {
        Arc::clone(&self.http_config)
    }
}
