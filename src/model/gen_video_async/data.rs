use serde::Serialize;
use validator::Validate;

use super::{
    super::traits::*,
    video_request::{Fps, ImageUrl, VideoBody, VideoDuration, VideoQuality, VideoSize},
};
use crate::client::ZaiClient;

/// Typed builder for submitting an asynchronous video-generation task.
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

    /// Set the video frame-rate selector.
    pub fn with_fps(mut self, fps: Fps) -> Self {
        self.body = self.body.with_fps(fps);
        self
    }

    /// Set the video-duration selector.
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
    /// `ChatCompletionResponse` (with `id`/`task_status`/`video_result`);
    /// poll it to completion via [`AsyncChatGetRequest`](crate::model::async_chat_get::AsyncChatGetRequest).
    pub async fn send_via(
        &self,
        client: &ZaiClient,
    ) -> crate::ZaiResult<crate::model::chat_base_response::ChatCompletionResponse>
    where
        N: serde::Serialize,
    {
        self.validate()?;
        let route = crate::client::routes::VIDEOS_GENERATE;
        let url = client.endpoints().resolve_route(route, &[])?;
        client
            .send_json::<_, crate::model::chat_base_response::ChatCompletionResponse>(
                route.method(),
                url,
                &self.body,
            )
            .await
    }
}
