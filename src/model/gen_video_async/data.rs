use std::sync::Arc;

use serde::Serialize;
use validator::Validate;

use super::{
    super::traits::*,
    video_request::{Fps, ImageUrl, VideoBody, VideoDuration, VideoQuality, VideoSize},
};
use crate::client::http::{HttpClientConfig, parse_typed_response, send_json_request};
use crate::client::v2::ZaiClient;

/// Video generation request structure
/// Handles HTTP requests for video generation API
///
/// (plan P05: migrated to route through [`ZaiClient`].)
pub struct VideoGenRequest<N>
where
    N: ModelName + VideoGen + Serialize,
{
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
    pub fn new(model: N) -> Self {
        let body = VideoBody::new(model);
        Self { body }
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

    /// Submit the video-generation request via a [`ZaiClient`] and parse the
    /// typed response.
    ///
    /// The async video endpoint returns a task-bearing body shaped like a
    /// [`ChatCompletionResponse`] (with `id`/`task_status`/`video_result`);
    /// poll it to completion via [`AsyncChatGetRequest`](crate::model::async_chat_get::AsyncChatGetRequest).
    pub async fn send_via(
        &self,
        client: &ZaiClient,
    ) -> crate::ZaiResult<crate::model::chat_base_response::ChatCompletionResponse>
    where
        N: serde::Serialize,
    {
        self.validate()?;
        let url = client.endpoints().resolve(
            crate::client::v2::ApiFamily::PaasV4,
            &["videos", "generations"],
        )?;
        let config = transport_config_from_client(client);
        let resp = send_json_request(
            reqwest::Method::POST,
            url,
            client.secret().expose(),
            &self.body,
            Arc::new(config),
        )
        .await?;
        parse_typed_response::<crate::model::chat_base_response::ChatCompletionResponse>(resp).await
    }
}

/// Build a legacy `HttpClientConfig` from a `ZaiClient`'s transport policy.
/// This is a temporary bridge during P05–P06; once all endpoints route through
/// the new `Transport`, this adapter is removed.
fn transport_config_from_client(client: &ZaiClient) -> HttpClientConfig {
    let t = client.transport();
    HttpClientConfig {
        timeout: std::time::Duration::from_secs(t.request_timeout.as_secs()),
        max_retries: u32::from(t.max_attempts).saturating_sub(1),
        enable_compression: t.enable_compression,
        retry_delay: crate::client::http::RetryDelay::Exponential {
            base: std::time::Duration::from_millis(500),
            max: std::time::Duration::from_secs(5),
        },
        enable_logging: false,
        mask_sensitive_data: true,
    }
}
