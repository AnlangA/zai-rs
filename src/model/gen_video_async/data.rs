use super::super::traits::*;
use super::video_request::{Fps, ImageUrl, VideoBody, VideoDuration, VideoQuality, VideoSize};
use crate::client::http::HttpClient;
use serde::Serialize;
use validator::Validate;

/// Video generation request structure
/// Handles HTTP requests for video generation API
pub struct VideoGenRequest<N>
where
    N: ModelName + VideoGen + Serialize,
{
    /// API key for authentication
    pub key: String,
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
        Self { key, body }
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
    pub fn validate(&self) -> anyhow::Result<()> {
        self.body.validate().map_err(|e| anyhow::anyhow!(e))?;
        Ok(())
    }
}

impl<N> HttpClient for VideoGenRequest<N>
where
    N: ModelName + VideoGen + Serialize,
{
    type Body = VideoBody<N>;
    /// API URL type
    type ApiUrl = &'static str;
    /// API key type
    type ApiKey = String;

    /// Get the API endpoint URL
    fn api_url(&self) -> &Self::ApiUrl {
        &"https://open.bigmodel.cn/api/paas/v4/videos/generations"
    }

    /// Get the API key for authentication
    fn api_key(&self) -> &Self::ApiKey {
        &self.key
    }

    /// Get the request body containing video generation parameters
    fn body(&self) -> &Self::Body {
        &self.body
    }
}
