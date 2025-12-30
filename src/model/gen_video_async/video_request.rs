use serde::Serialize;
use validator::*;

use super::super::traits::*;

#[derive(Debug, Clone, Validate, Serialize)]
#[validate(schema(function = "validate_prompt_or_image"))]
pub struct VideoBody<N>
where
    N: ModelName + Serialize,
{
    /// Model identifier for video generation API
    pub model: N,
    /// Image URL(s) for video generation base
    /// Supports single URL string or array of URLs (1-2 URLs)
    /// Supported formats: .png, .jpeg, .jpg, max 5MB
    /// Either prompt or image_url must be provided (or both)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_url: Option<ImageUrl>,
    /// Text description for video generation, max 1500 characters
    /// Either prompt or image_url must be provided (or both)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(length(max = 1500))]
    pub prompt: Option<String>,
    /// Output quality mode, defaults to "speed"
    /// "quality": prioritize higher generation quality
    /// "speed": prioritize faster generation with slightly lower quality
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quality: Option<VideoQuality>,
    /// Whether to generate AI audio effects, defaults to false
    #[serde(skip_serializing_if = "Option::is_none")]
    pub with_audio: Option<bool>,
    /// Control watermark for AI-generated content
    /// true: enable watermarks to meet policy requirements
    /// false: disable watermarks, only for authorized customers
    #[serde(skip_serializing_if = "Option::is_none")]
    pub watermark_enabled: Option<bool>,
    /// Video resolution size
    /// If not specified, short side defaults to 1080, long side determined by
    /// aspect ratio Supports up to 4K resolution
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<VideoSize>,
    /// Video frame rate (FPS), supported values: 30 or 60, defaults to 30
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fps: Option<Fps>,
    /// Video duration in seconds, defaults to 5, supported: 5 or 10
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<VideoDuration>,
    /// Unique request identifier provided by client
    /// If not provided, platform will generate one automatically
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    /// End user's unique ID for policy enforcement
    /// Length requirements: minimum 6 characters, maximum 128 characters
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(length(min = 6, max = 128))]
    pub user_id: Option<String>,
}

impl<N> VideoBody<N>
where
    N: ModelName + Serialize,
{
    /// Create a new VideoBody with the specified model
    pub fn new(model: N) -> Self {
        Self {
            model,
            prompt: None,
            quality: None,
            with_audio: None,
            watermark_enabled: None,
            image_url: None,
            size: None,
            fps: None,
            duration: None,
            request_id: None,
            user_id: None,
        }
    }

    /// Set the text prompt for video generation
    pub fn with_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.prompt = Some(prompt.into());
        self
    }

    /// Set the quality mode (speed or quality)
    pub fn with_quality(mut self, quality: VideoQuality) -> Self {
        self.quality = Some(quality);
        self
    }

    /// Enable/disable audio generation
    pub fn with_audio(mut self, with_audio: bool) -> Self {
        self.with_audio = Some(with_audio);
        self
    }

    /// Enable/disable watermark
    pub fn with_watermark_enabled(mut self, watermark_enabled: bool) -> Self {
        self.watermark_enabled = Some(watermark_enabled);
        self
    }

    /// Set image URL(s) for video generation
    pub fn with_image_url(mut self, image_url: ImageUrl) -> Self {
        self.image_url = Some(image_url);
        self
    }

    /// Set video resolution size
    pub fn with_size(mut self, size: VideoSize) -> Self {
        self.size = Some(size);
        self
    }

    /// Set video frame rate (30 or 60 FPS)
    pub fn with_fps(mut self, fps: Fps) -> Self {
        self.fps = Some(fps);
        self
    }

    /// Set video duration (5 or 10 seconds)
    pub fn with_duration(mut self, duration: VideoDuration) -> Self {
        self.duration = Some(duration);
        self
    }

    /// Set custom request ID
    pub fn with_request_id(mut self, request_id: impl Into<String>) -> Self {
        self.request_id = Some(request_id.into());
        self
    }

    /// Set user ID for policy enforcement
    pub fn with_user_id(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = Some(user_id.into());
        self
    }

    /// Create a video request with prompt only (Format 1)
    pub fn prompt_only(model: N, prompt: impl Into<String>) -> Self {
        Self::new(model).with_prompt(prompt)
    }

    /// Create a video request with single image URL and prompt (Format 2)
    pub fn with_single_image(
        model: N,
        image_url: impl Into<String>,
        prompt: impl Into<String>,
    ) -> Self {
        Self::new(model)
            .with_image_url(ImageUrl::from_url(image_url))
            .with_prompt(prompt)
    }

    /// Create a video request with multiple image URLs and prompt (Format 3)
    pub fn with_multiple_images(
        model: N,
        mut image_urls: Vec<impl Into<String>>,
        prompt: impl Into<String>,
    ) -> Self {
        let image_url = if image_urls.len() == 1 {
            ImageUrl::from_url(image_urls.remove(0))
        } else if image_urls.len() == 2 {
            ImageUrl::from_two_urls(image_urls.remove(0), image_urls.remove(0))
        } else {
            panic!("with_multiple_images requires 1 or 2 URLs");
        };

        Self::new(model)
            .with_image_url(image_url)
            .with_prompt(prompt)
    }
}

// Struct-level validation: require at least one of prompt or image_url.
#[allow(dead_code)]
fn validate_prompt_or_image<N>(body: &VideoBody<N>) -> Result<(), validator::ValidationError>
where
    N: ModelName + Serialize,
{
    let has_prompt = body.prompt.as_ref().map(|s| !s.is_empty()).unwrap_or(false);
    let has_image = body.image_url.is_some();
    if has_prompt || has_image {
        Ok(())
    } else {
        Err(validator::ValidationError::new("prompt_or_image_required"))
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum VideoQuality {
    /// Prioritize faster generation with slightly lower quality
    Speed,
    /// Prioritize higher generation quality
    Quality,
}

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum ImageUrl {
    /// Base64 encoded image data
    Base64(String),
    /// Single URL or array of URLs (1-2 URLs)
    VecUrl(Vec<String>),
}

impl ImageUrl {
    /// Create ImageUrl from base64-encoded data
    pub fn base64(data: impl Into<String>) -> Self {
        ImageUrl::Base64(data.into())
    }

    /// Create ImageUrl from a single URL
    pub fn from_url(url: impl Into<String>) -> Self {
        ImageUrl::VecUrl(vec![url.into()])
    }

    /// Create ImageUrl from exactly two URLs
    pub fn from_two_urls(u1: impl Into<String>, u2: impl Into<String>) -> Self {
        ImageUrl::VecUrl(vec![u1.into(), u2.into()])
    }
}

#[derive(Debug, Clone, Serialize)]
pub enum VideoSize {
    /// 1280x720 resolution (HD)
    #[serde(rename = "1280x720")]
    Size1280x720,
    /// 720x1280 resolution (vertical HD)
    #[serde(rename = "720x1280")]
    Size720x1280,
    /// 1024x1024 resolution (square)
    #[serde(rename = "1024x1024")]
    Size1024x1024,
    /// 1920x1080 resolution (Full HD)
    #[serde(rename = "1920x1080")]
    Size1920x1080,
    /// 1080x1920 resolution (vertical Full HD)
    #[serde(rename = "1080x1920")]
    Size1080x1920,
    /// 2048x1080 resolution (2K)
    #[serde(rename = "2048x1080")]
    Size2048x1080,
    /// 3840x2160 resolution (4K)
    #[serde(rename = "3840x2160")]
    Size3840x2160,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Fps {
    /// 30 frames per second
    Fps30,
    /// 60 frames per second
    Fps60,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum VideoDuration {
    /// 5 seconds duration
    Duration5,
    /// 10 seconds duration
    Duration10,
}
