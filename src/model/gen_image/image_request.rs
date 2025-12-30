use serde::Serialize;
use validator::Validate;

use super::super::traits::*;

/// Request body for image generation
#[derive(Debug, Clone, Serialize, Validate)]
pub struct ImageGenBody<N>
where
    N: ModelName + ImageGen + Serialize,
{
    /// The model to use for image generation
    pub model: N,
    /// Text description of the desired image
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(length(min = 1, max = 4000))]
    pub prompt: Option<String>,
    /// Image generation quality
    /// - HD: generates more refined and detailed images with higher
    ///   consistency, takes ~20 seconds
    /// - Standard: fast image generation, suitable for scenarios requiring
    ///   speed, takes ~5-10 seconds This parameter only supports
    ///   cogview-4-250304
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quality: Option<ImageQuality>,
    /// Image size
    /// Recommended values: 1024x1024 (default), 768x1344, 864x1152, 1344x768,
    /// 1152x864, 1440x720, 720x1440 Custom dimensions: width and height
    /// must be between 512-2048px, divisible by 16, and total pixels must
    /// not exceed 2^21 (2,097,152)
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

/// Image generation quality options
#[derive(Debug, Clone, Serialize)]
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
/// Recommended sizes:
/// - 1024x1024 (default)
/// - 768x1344
/// - 864x1152
/// - 1344x768
/// - 1152x864
/// - 1440x720
/// - 720x1440
///
/// Custom sizes must satisfy:
/// - Width and height between 512-2048px
/// - Both dimensions divisible by 16
/// - Total pixels <= 2^21 (2,097,152)
#[derive(Debug, Clone)]
pub enum ImageSize {
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
    Custom { width: u32, height: u32 },
}

impl serde::Serialize for ImageSize {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let s = match self {
            ImageSize::Size1024x1024 => "1024x1024".to_string(),
            ImageSize::Size768x1344 => "768x1344".to_string(),
            ImageSize::Size864x1152 => "864x1152".to_string(),
            ImageSize::Size1344x768 => "1344x768".to_string(),
            ImageSize::Size1152x864 => "1152x864".to_string(),
            ImageSize::Size1440x720 => "1440x720".to_string(),
            ImageSize::Size720x1440 => "720x1440".to_string(),
            ImageSize::Custom { width, height } => format!("{}x{}", width, height),
        };
        serializer.serialize_str(&s)
    }
}

impl ImageSize {
    /// Validate image size constraints
    ///
    /// Returns true if the size meets all requirements:
    /// - Dimensions between 512-2048px
    /// - Both dimensions divisible by 16
    /// - Total pixels <= 2^21 (2,097,152)
    pub fn is_valid(&self) -> bool {
        match self {
            ImageSize::Custom { width, height } => {
                // Check dimension range
                if *width < 512 || *width > 2048 || *height < 512 || *height > 2048 {
                    return false;
                }

                // Check divisibility by 16
                if width % 16 != 0 || height % 16 != 0 {
                    return false;
                }

                // Check total pixels limit (2^21 = 2,097,152)
                let total_pixels = (*width as u64) * (*height as u64);
                total_pixels <= 2_097_152
            },
            _ => true, // Predefined sizes are already valid
        }
    }

    /// Get the dimensions as (width, height)
    pub fn dimensions(&self) -> (u32, u32) {
        match self {
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
