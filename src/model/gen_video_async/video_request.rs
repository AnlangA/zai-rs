use serde::Serialize;
use validator::*;

use super::super::traits::*;

/// Request body for asynchronous video generation.
#[derive(Debug, Clone, Validate, Serialize)]
#[validate(schema(function = "validate_prompt_or_image"))]
pub struct VideoBody<N>
where
    N: VideoGen,
{
    /// Model identifier for video generation API
    pub model: N,
    /// Image input used as the video-generation source.
    ///
    /// A single URL or pre-encoded image serializes as a string. A first/last
    /// frame pair serializes as a two-element array. Either this field or
    /// `prompt` must be set.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_url: Option<ImageUrl>,
    /// Text description for video generation, at most 512 characters.
    /// Either prompt or image_url must be provided (or both)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(length(max = 512))]
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
    /// Video frame-rate selector (`30` or `60` on the wire).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fps: Option<Fps>,
    /// Video-duration selector (`5` or `10` on the wire).
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
    N: VideoGen,
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

    /// Set the video frame-rate selector.
    pub fn with_fps(mut self, fps: Fps) -> Self {
        self.fps = Some(fps);
        self
    }

    /// Set the video-duration selector.
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
    ///
    /// # Errors
    /// Returns an error if `image_urls` does not contain exactly 1 or 2 URLs.
    pub fn with_multiple_images(
        model: N,
        image_urls: Vec<impl Into<String>>,
        prompt: impl Into<String>,
    ) -> Result<Self, crate::ZaiError> {
        let count = image_urls.len();
        if !(1..=2).contains(&count) {
            return Err(crate::ZaiError::ApiError {
                code: crate::client::error::codes::SDK_VALIDATION,
                message: "with_multiple_images requires 1 or 2 URLs".to_string(),
            });
        }
        let mut image_urls = image_urls.into_iter();
        let first = image_urls.next().ok_or_else(|| crate::ZaiError::ApiError {
            code: crate::client::error::codes::SDK_VALIDATION,
            message: "with_multiple_images requires 1 or 2 URLs".to_string(),
        })?;
        let image_url = match image_urls.next() {
            Some(second) => ImageUrl::from_two_urls(first, second),
            None => ImageUrl::from_url(first),
        };

        Ok(Self::new(model)
            .with_image_url(image_url)
            .with_prompt(prompt))
    }

    /// Validate this VideoBody before sending.
    ///
    /// Ensures that at least one of `prompt` or `image_url` is present,
    /// and runs all field-level validations (prompt length, user_id length,
    /// etc.).
    pub fn validate_body(&self) -> crate::ZaiResult<()> {
        self.validate()
            .map_err(crate::client::error::ZaiError::from)
    }
}

fn validate_prompt_or_image<N>(body: &VideoBody<N>) -> Result<(), validator::ValidationError>
where
    N: VideoGen,
{
    let has_prompt = body.prompt.is_some();
    if body
        .prompt
        .as_deref()
        .is_some_and(|prompt| prompt.trim().is_empty())
    {
        return Err(validator::ValidationError::new("prompt_must_not_be_blank"));
    }

    let image_is_valid = match body.image_url.as_ref() {
        Some(ImageUrl::Url(url)) => is_http_url(url),
        Some(ImageUrl::Base64(data)) => is_image_data_url(data),
        Some(ImageUrl::VecUrl(urls)) => urls.len() == 2 && urls.iter().all(|url| is_http_url(url)),
        None => true,
    };
    if !image_is_valid {
        return Err(validator::ValidationError::new("invalid_image_url"));
    }
    if !has_prompt && body.image_url.is_none() {
        return Err(validator::ValidationError::new("prompt_or_image_required"));
    }
    Ok(())
}

fn is_http_url(value: &str) -> bool {
    value
        .parse::<url::Url>()
        .is_ok_and(|url| matches!(url.scheme(), "http" | "https"))
}

fn is_image_data_url(value: &str) -> bool {
    let Some((metadata, payload)) = value.trim().split_once(',') else {
        return false;
    };
    metadata.starts_with("data:image/")
        && metadata.ends_with(";base64")
        && !payload.trim().is_empty()
}

/// Output quality mode for video generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum VideoQuality {
    /// Prioritize faster generation with slightly lower quality
    Speed,
    /// Prioritize higher generation quality
    Quality,
}

/// Image input for video generation: a pre-encoded string or one/two URLs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum ImageUrl {
    /// One HTTP(S) image URL.
    Url(String),
    /// Base64-encoded image data URL; sent unchanged.
    Base64(String),
    /// First- and last-frame URLs, serialized as a two-element JSON array.
    VecUrl(Vec<String>),
}

impl ImageUrl {
    /// Wrap a pre-encoded image string without modifying it.
    pub fn base64(data: impl Into<String>) -> Self {
        ImageUrl::Base64(data.into())
    }

    /// Wrap one HTTP(S) URL as a JSON string.
    pub fn from_url(url: impl Into<String>) -> Self {
        ImageUrl::Url(url.into())
    }

    /// Wrap exactly two URLs as a JSON array.
    pub fn from_two_urls(u1: impl Into<String>, u2: impl Into<String>) -> Self {
        ImageUrl::VecUrl(vec![u1.into(), u2.into()])
    }
}

/// Supported video resolution presets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
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

/// Video frame rate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fps {
    /// 30 frames per second.
    Fps30,
    /// 60 frames per second.
    Fps60,
}

impl Serialize for Fps {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_u8(match self {
            Self::Fps30 => 30,
            Self::Fps60 => 60,
        })
    }
}

/// Video duration in seconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoDuration {
    /// Five seconds.
    Duration5,
    /// Ten seconds.
    Duration10,
}

impl Serialize for VideoDuration {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_u8(match self {
            Self::Duration5 => 5,
            Self::Duration10 => 10,
        })
    }
}
