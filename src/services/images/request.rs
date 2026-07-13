//! Typed request for asynchronous GLM-Image generation.

use serde::{Deserialize, Serialize};

use crate::{
    ZaiResult,
    client::{
        ZaiClient,
        error::{ZaiError, codes},
    },
    model::AsyncResponse,
};

fn validation_error(message: impl Into<String>) -> ZaiError {
    ZaiError::ApiError {
        code: codes::SDK_VALIDATION,
        message: message.into(),
    }
}

fn valid_glm_image_size(value: &str) -> bool {
    if matches!(value, "1728x960" | "960x1728") {
        return true;
    }
    let Some((width, height)) = value.split_once('x') else {
        return false;
    };
    let (Ok(width), Ok(height)) = (width.parse::<u32>(), height.parse::<u32>()) else {
        return false;
    };
    (1024..=2048).contains(&width)
        && (1024..=2048).contains(&height)
        && width.is_multiple_of(32)
        && height.is_multiple_of(32)
        && u64::from(width) * u64::from(height) <= 1 << 22
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
enum AsyncImageModel {
    #[serde(rename = "glm-image")]
    GlmImage,
}

/// Image quality accepted by the frozen async GLM-Image schema.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AsyncImageQuality {
    /// High-definition generation.
    #[serde(rename = "hd")]
    Hd,
}

/// Request for `POST /async/images/generations`.
#[derive(Clone, Serialize)]
pub struct AsyncImageGenerationRequest {
    model: AsyncImageModel,
    prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    quality: Option<AsyncImageQuality>,
    #[serde(skip_serializing_if = "Option::is_none")]
    size: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    watermark_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    user_id: Option<String>,
}

impl std::fmt::Debug for AsyncImageGenerationRequest {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AsyncImageGenerationRequest")
            .field("model", &self.model())
            .field("prompt", &"[REDACTED]")
            .field("quality", &self.quality)
            .field("size", &self.size)
            .field("watermark_enabled", &self.watermark_enabled)
            .field("user_id", &self.user_id.as_ref().map(|_| "[REDACTED]"))
            .finish()
    }
}

impl AsyncImageGenerationRequest {
    /// Create a GLM-Image request from the required prompt.
    pub fn new(prompt: impl Into<String>) -> Self {
        Self {
            model: AsyncImageModel::GlmImage,
            prompt: prompt.into(),
            quality: None,
            size: None,
            watermark_enabled: None,
            user_id: None,
        }
    }

    /// Select the image quality.
    pub fn with_quality(mut self, quality: AsyncImageQuality) -> Self {
        self.quality = Some(quality);
        self
    }

    /// Set a recommended or valid custom image size.
    pub fn with_size(mut self, size: impl Into<String>) -> Self {
        self.size = Some(size.into());
        self
    }

    /// Configure generated-image watermarking.
    pub fn with_watermark(mut self, enabled: bool) -> Self {
        self.watermark_enabled = Some(enabled);
        self
    }

    /// Set the end-user identifier used for abuse monitoring.
    pub fn with_user_id(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = Some(user_id.into());
        self
    }

    /// Return the fixed model identifier sent on the wire.
    pub const fn model(&self) -> &'static str {
        "glm-image"
    }

    /// Borrow the image prompt.
    pub fn prompt(&self) -> &str {
        &self.prompt
    }

    /// Return the configured quality.
    pub const fn quality(&self) -> Option<AsyncImageQuality> {
        self.quality
    }

    /// Borrow the configured image size.
    pub fn size(&self) -> Option<&str> {
        self.size.as_deref()
    }

    /// Return the configured watermark flag.
    pub const fn watermark_enabled(&self) -> Option<bool> {
        self.watermark_enabled
    }

    /// Borrow the configured end-user identifier.
    pub fn user_id(&self) -> Option<&str> {
        self.user_id.as_deref()
    }

    /// Validate documented prompt, size, and user-id constraints.
    pub fn validate(&self) -> ZaiResult<()> {
        if self.prompt.trim().is_empty() {
            return Err(validation_error("async image prompt must not be blank"));
        }
        if self
            .size
            .as_deref()
            .is_some_and(|size| !valid_glm_image_size(size))
        {
            return Err(validation_error("invalid async GLM-Image size"));
        }
        if self
            .user_id
            .as_ref()
            .is_some_and(|value| !(6..=128).contains(&value.chars().count()))
        {
            return Err(validation_error(
                "async image user_id must contain between 6 and 128 characters",
            ));
        }
        Ok(())
    }

    /// Validate, send, and decode the async task response.
    pub async fn send_via(&self, client: &ZaiClient) -> ZaiResult<AsyncResponse> {
        self.validate()?;
        let route = crate::client::routes::IMAGES_GENERATE_ASYNC;
        let response = client
            .operation(route)
            .send_json::<_, AsyncResponse>(self)
            .await?;
        response.validate()?;
        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_serializes_the_fixed_model_and_closed_fields() {
        let request = AsyncImageGenerationRequest::new("a cat")
            .with_quality(AsyncImageQuality::Hd)
            .with_size("960x1728")
            .with_watermark(true)
            .with_user_id("user-1");
        assert_eq!(
            serde_json::to_value(request).unwrap(),
            serde_json::json!({
                "model": "glm-image",
                "prompt": "a cat",
                "quality": "hd",
                "size": "960x1728",
                "watermark_enabled": true,
                "user_id": "user-1"
            })
        );
        let debug = format!(
            "{:?}",
            AsyncImageGenerationRequest::new("private prompt").with_user_id("private-user")
        );
        assert!(!debug.contains("private prompt"));
        assert!(!debug.contains("private-user"));
    }

    #[test]
    fn validation_rejects_blank_prompts_and_invalid_documented_values() {
        assert!(AsyncImageGenerationRequest::new(" ").validate().is_err());
        assert!(
            AsyncImageGenerationRequest::new("cat")
                .with_size("1025x1024")
                .validate()
                .is_err()
        );
        assert!(
            AsyncImageGenerationRequest::new("cat")
                .with_user_id("short")
                .validate()
                .is_err()
        );
    }
}
