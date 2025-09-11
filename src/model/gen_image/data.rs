use super::super::traits::*;
use super::image_request::{ImageGenBody, ImageQuality, ImageSize};
use crate::client::http::HttpClient;
use serde::Serialize;

/// Image generation request structure
/// Provides a typed builder around the image generation API body
pub struct ImageGenRequest<N>
where
    N: ModelName + ImageGen + Serialize,
{
    /// API key for authentication
    pub key: String,
    /// Request body
    body: ImageGenBody<N>,
}

impl<N> ImageGenRequest<N>
where
    N: ModelName + ImageGen + Serialize,
{
    /// Create a new image generation request for the given model and API key
    pub fn new(model: N, key: String) -> Self {
        let body = ImageGenBody {
            model,
            prompt: None,
            quality: None,
            size: None,
            watermark_enabled: None,
            user_id: None,
        };
        Self { key, body }
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
}

impl<N> HttpClient for ImageGenRequest<N>
where
    N: ModelName + ImageGen + Serialize,
{
    type Body = ImageGenBody<N>;
    type ApiUrl = &'static str;
    type ApiKey = String;

    fn api_url(&self) -> &Self::ApiUrl {
        &"https://open.bigmodel.cn/api/paas/v4/images/generations"
    }

    fn api_key(&self) -> &Self::ApiKey {
        &self.key
    }

    fn body(&self) -> &Self::Body {
        &self.body
    }
}

