use serde::Serialize;
use validator::Validate;

use super::super::traits::*;

/// Request body for image generation
#[derive(Clone, Serialize, Validate)]
#[validate(schema(function = "validate_image_body"))]
pub struct ImageGenBody<N>
where
    N: ImageGen,
{
    /// The model to use for image generation
    pub model: N,
    /// Text description of the desired image
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(length(min = 1))]
    pub prompt: Option<String>,
    /// Image-generation quality.
    ///
    /// `Hd` favors detail and consistency; `Standard` favors lower latency.
    /// The `glm-image` model supports only [`ImageQuality::Hd`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quality: Option<ImageQuality>,
    /// Image size
    /// Use an [`ImageSize`] preset or custom dimensions satisfying the limits
    /// documented on that type.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<ImageSize>,
    /// Whether to add watermark to AI generated images
    #[serde(skip_serializing_if = "Option::is_none")]
    pub watermark_enabled: Option<bool>,
    /// Unique ID of the end user to help platform intervene against violations,
    /// illegal content generation, or other abusive behaviors
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(length(min = 6, max = 128))]
    pub user_id: Option<String>,
}

impl<N> std::fmt::Debug for ImageGenBody<N>
where
    N: ImageGen + std::fmt::Debug,
{
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ImageGenBody")
            .field("model", &self.model)
            .field("prompt_configured", &self.prompt.is_some())
            .field("quality", &self.quality)
            .field("size", &self.size)
            .field("watermark_enabled", &self.watermark_enabled)
            .field("user_id", &self.user_id.as_ref().map(|_| "[REDACTED]"))
            .finish()
    }
}

fn validate_image_body<N>(body: &ImageGenBody<N>) -> Result<(), validator::ValidationError>
where
    N: ImageGen,
{
    if body
        .prompt
        .as_deref()
        .is_none_or(|prompt| prompt.trim().is_empty())
    {
        return Err(validator::ValidationError::new("prompt_required"));
    }
    let model = N::NAME;

    if model == "glm-image" && matches!(body.quality, Some(ImageQuality::Standard)) {
        return Err(validator::ValidationError::new(
            "quality_not_supported_by_model",
        ));
    }
    if body
        .size
        .as_ref()
        .is_some_and(|size| !size.is_valid_for_model(model))
    {
        return Err(validator::ValidationError::new("invalid_image_size"));
    }
    Ok(())
}

impl<N> ImageGenBody<N>
where
    N: ImageGen,
{
    /// Create an image-generation body with all optional fields omitted.
    pub fn new(model: N) -> Self {
        Self {
            model,
            prompt: None,
            quality: None,
            size: None,
            watermark_enabled: None,
            user_id: None,
        }
    }
}

/// Image generation quality options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ImageQuality {
    /// High quality - more refined and detailed images
    #[serde(rename = "hd")]
    Hd,
    /// Standard quality - faster generation
    #[serde(rename = "standard")]
    Standard,
}

/// Image size options
///
/// `glm-image` accepts dimensions from 1,024 through 2,048 pixels, divisible
/// by 32, with at most 2²² total pixels. Other models accept dimensions from
/// 512 through 2,048 pixels, divisible by 16, with at most 2²¹ total pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ImageSize {
    /// 1280x1280 pixels (`glm-image` default)
    Size1280x1280,
    /// 1568x1056 pixels
    Size1568x1056,
    /// 1056x1568 pixels
    Size1056x1568,
    /// 1472x1088 pixels
    Size1472x1088,
    /// 1088x1472 pixels
    Size1088x1472,
    /// 1728x960 pixels
    Size1728x960,
    /// 960x1728 pixels
    Size960x1728,
    /// 1024x1024 pixels
    Size1024x1024,
    /// 768x1344 pixels
    Size768x1344,
    /// 864x1152 pixels
    Size864x1152,
    /// 1344x768 pixels
    Size1344x768,
    /// 1152x864 pixels
    Size1152x864,
    /// 1440x720 pixels
    Size1440x720,
    /// 720x1440 pixels
    Size720x1440,
    /// Custom dimensions in width x height format
    Custom {
        /// Custom width in pixels; limits depend on the selected model.
        width: u32,
        /// Custom height in pixels; limits depend on the selected model.
        height: u32,
    },
}

impl serde::Serialize for ImageSize {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let dimensions = match self {
            ImageSize::Size1280x1280 => "1280x1280",
            ImageSize::Size1568x1056 => "1568x1056",
            ImageSize::Size1056x1568 => "1056x1568",
            ImageSize::Size1472x1088 => "1472x1088",
            ImageSize::Size1088x1472 => "1088x1472",
            ImageSize::Size1728x960 => "1728x960",
            ImageSize::Size960x1728 => "960x1728",
            ImageSize::Size1024x1024 => "1024x1024",
            ImageSize::Size768x1344 => "768x1344",
            ImageSize::Size864x1152 => "864x1152",
            ImageSize::Size1344x768 => "1344x768",
            ImageSize::Size1152x864 => "1152x864",
            ImageSize::Size1440x720 => "1440x720",
            ImageSize::Size720x1440 => "720x1440",
            ImageSize::Custom { width, height } => {
                return serializer.collect_str(&format_args!("{width}x{height}"));
            },
        };
        serializer.serialize_str(dimensions)
    }
}

impl ImageSize {
    /// Validate the size against the non-`glm-image` constraints.
    ///
    /// This method retains the original SDK behavior. Request-body validation
    /// automatically applies the correct rules for the selected model.
    pub fn is_valid(&self) -> bool {
        let (width, height) = self.dimensions();
        dimensions_are_valid(width, height, 512, 16, 1 << 21)
    }

    fn is_valid_for_model(&self, model: &str) -> bool {
        if model == "glm-image"
            && matches!(
                self,
                Self::Size1280x1280
                    | Self::Size1568x1056
                    | Self::Size1056x1568
                    | Self::Size1472x1088
                    | Self::Size1088x1472
                    | Self::Size1728x960
                    | Self::Size960x1728
            )
        {
            return true;
        }
        let (width, height) = self.dimensions();
        if model == "glm-image" {
            dimensions_are_valid(width, height, 1024, 32, 1 << 22)
        } else {
            dimensions_are_valid(width, height, 512, 16, 1 << 21)
        }
    }

    /// Get the dimensions as (width, height)
    pub fn dimensions(&self) -> (u32, u32) {
        match self {
            ImageSize::Size1280x1280 => (1280, 1280),
            ImageSize::Size1568x1056 => (1568, 1056),
            ImageSize::Size1056x1568 => (1056, 1568),
            ImageSize::Size1472x1088 => (1472, 1088),
            ImageSize::Size1088x1472 => (1088, 1472),
            ImageSize::Size1728x960 => (1728, 960),
            ImageSize::Size960x1728 => (960, 1728),
            ImageSize::Size1024x1024 => (1024, 1024),
            ImageSize::Size768x1344 => (768, 1344),
            ImageSize::Size864x1152 => (864, 1152),
            ImageSize::Size1344x768 => (1344, 768),
            ImageSize::Size1152x864 => (1152, 864),
            ImageSize::Size1440x720 => (1440, 720),
            ImageSize::Size720x1440 => (720, 1440),
            ImageSize::Custom { width, height } => (*width, *height),
        }
    }
}

fn dimensions_are_valid(
    width: u32,
    height: u32,
    minimum: u32,
    divisor: u32,
    maximum_pixels: u64,
) -> bool {
    (minimum..=2048).contains(&width)
        && (minimum..=2048).contains(&height)
        && width.is_multiple_of(divisor)
        && height.is_multiple_of(divisor)
        && u64::from(width) * u64::from(height) <= maximum_pixels
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::gen_image::{CogView4, GlmImage};

    #[test]
    fn body_validation_requires_prompt_and_valid_custom_size() {
        assert!(ImageGenBody::new(CogView4 {}).validate().is_err());
        let mut body = ImageGenBody::new(CogView4 {});
        body.prompt = Some("image".into());
        assert!(body.validate().is_ok());
        body.size = Some(ImageSize::Custom {
            width: 513,
            height: 512,
        });
        assert!(body.validate().is_err());
    }

    #[test]
    fn body_validation_applies_model_specific_size_and_quality_rules() {
        let mut glm = ImageGenBody::new(GlmImage {});
        glm.prompt = Some("image".into());
        glm.size = Some(ImageSize::Size960x1728);
        glm.quality = Some(ImageQuality::Hd);
        assert!(glm.validate().is_ok());

        glm.quality = Some(ImageQuality::Standard);
        assert!(glm.validate().is_err());

        let mut cogview = ImageGenBody::new(CogView4 {});
        cogview.prompt = Some("image".into());
        cogview.size = Some(ImageSize::Size1568x1056);
        assert!(cogview.validate().is_ok());
        cogview.size = Some(ImageSize::Size1280x1280);
        assert!(cogview.validate().is_ok());
    }

    #[test]
    fn debug_redacts_prompt_and_user_identifier() {
        let mut body = ImageGenBody::new(GlmImage {});
        body.prompt = Some("private image prompt".to_owned());
        body.user_id = Some("private-user".to_owned());
        let debug = format!("{body:?}");
        assert!(!debug.contains("private image prompt"));
        assert!(!debug.contains("private-user"));
        assert!(debug.contains("prompt_configured: true"));
    }
}
