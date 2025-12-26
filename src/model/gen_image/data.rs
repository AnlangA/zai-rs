use super::super::traits::*;
use super::image_request::{ImageGenBody, ImageQuality, ImageSize};
use crate::client::http::HttpClient;
use serde::Serialize;
use validator::Validate;

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

    pub fn validate(&self) -> crate::ZaiResult<()> {
        // Body-level field validations
        self.body
            .validate()
            .map_err(|e| crate::client::error::ZaiError::ApiError {
                code: 1200,
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
                code: 1200,
                message: "prompt is required".to_string(),
            });
        }
        // Validate custom size when present
        if let Some(size) = &self.body.size
            && let super::image_request::ImageSize::Custom { .. } = size
                && !size.is_valid() {
                    return Err(crate::client::error::ZaiError::ApiError {
                        code: 1200,
                        message: "invalid custom image size: must be 512..=2048, divisible by 16, and <= 2^21 pixels".to_string(),
                    });
                }
        Ok(())
    }

    pub async fn send(&self) -> crate::ZaiResult<super::image_response::ImageResponse> {
        self.validate()?;
        let resp = self.post().await?;
        let parsed = resp.json::<super::image_response::ImageResponse>().await?;
        Ok(parsed)
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
