use super::super::traits::*;
use super::video_request::VideoBody;
use crate::client::http::HttpClient;
use serde::Serialize;

/// Video generation request structure
/// Handles HTTP requests for video generation API
pub struct VideoGenRequest<N>
where
    N: ModelName + VideoGen + Serialize,
{
    /// API key for authentication
    pub key: String,
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
    /// * `body` - Video generation parameters and configuration
    /// * `key` - API key for authentication
    pub fn new(model: N, key: String) -> Self {
        let body = VideoBody::new(model);
        Self { key, body }
    }
}

impl<N> HttpClient for VideoGenRequest<N>
where
    N: ModelName + VideoGen + Serialize,
{
    type Body = VideoBody<N>;
    /// API URL type
    type ApiUrl = &'static str;
    /// API key type
    type ApiKey = String;

    /// Get the API endpoint URL
    fn api_url(&self) -> &Self::ApiUrl {
        &"https://open.bigmodel.cn/api/paas/v4/videos/generations"
    }

    /// Get the API key for authentication
    fn api_key(&self) -> &Self::ApiKey {
        &self.key
    }

    /// Get the request body containing video generation parameters
    fn body(&self) -> &Self::Body {
        &self.body
    }
}
