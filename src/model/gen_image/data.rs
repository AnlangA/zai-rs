use validator::Validate;

use super::{
    super::traits::*,
    image_request::{ImageGenBody, ImageQuality, ImageSize},
};
use crate::client::ZaiClient;

/// Typed builder for an image-generation request sent through a [`ZaiClient`].
pub struct ImageGenRequest<N>
where
    N: ImageGen,
{
    /// Request body
    body: ImageGenBody<N>,
}

impl<N> ImageGenRequest<N>
where
    N: ImageGen,
{
    /// Create a new image generation request for the given model.
    pub fn new(model: N) -> Self {
        Self {
            body: ImageGenBody::new(model),
        }
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
        self.body
            .validate()
            .map_err(crate::client::error::ZaiError::from)?;
        Ok(())
    }

    /// Submit the request via a [`ZaiClient`] and parse the typed
    /// image-generation response.
    pub async fn send_via(
        &self,
        client: &ZaiClient,
    ) -> crate::ZaiResult<super::image_response::ImageResponse> {
        self.validate()?;
        let route = crate::client::routes::IMAGES_GENERATE;
        client
            .operation(route)
            .send_json::<_, super::image_response::ImageResponse>(&self.body)
            .await
    }
}
