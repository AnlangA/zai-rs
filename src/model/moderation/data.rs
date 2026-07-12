//! # Content Moderation API
//!
//! This module provides the content moderation client for analyzing text,
//! image, audio, and video content for safety risks.

use super::models::*;
use crate::client::ZaiClient;

/// Content moderation client.
///
/// This client provides functionality to moderate content for safety risks,
/// supporting text, image, audio, and video formats. Credentials and
/// transport live on a [`ZaiClient`], passed to
/// [`send_via`](Self::send_via).
pub struct Moderation {
    /// Moderation request body
    body: ModerationRequest,
}

impl Moderation {
    /// Creates a new moderation request with text content.
    ///
    /// ## Arguments
    ///
    /// * `text` - Text to moderate; the current validator permits at most 2,000
    ///   UTF-8 bytes
    ///
    /// ## Returns
    ///
    /// A new `Moderation` instance configured for text moderation.
    pub fn new_text(text: impl Into<String>) -> Self {
        let body = ModerationRequest::new_text(text);
        Self { body }
    }

    /// Creates a new moderation request with multimedia content.
    ///
    /// ## Arguments
    ///
    /// * `content_type` - The type of multimedia content (image, audio, video)
    /// * `url` - URL to the multimedia content
    ///
    /// ## Returns
    ///
    /// A new `Moderation` instance configured for multimedia moderation.
    pub fn new_multimedia(content_type: MediaType, url: impl Into<String>) -> Self {
        let body = ModerationRequest::new_multimedia(content_type, url);
        Self { body }
    }

    /// Gets mutable access to the request body for further customization.
    pub fn body_mut(&mut self) -> &mut ModerationRequest {
        &mut self.body
    }

    /// Validate the request body constraints before sending.
    pub fn validate(&self) -> crate::ZaiResult<()> {
        self.body
            .validate()
            .map_err(crate::client::error::ZaiError::from)?;
        Ok(())
    }

    /// Sends the moderation request via a [`ZaiClient`] and returns the
    /// structured response.
    ///
    /// This method automatically validates the request before sending.
    ///
    /// ## Returns
    ///
    /// A `ModerationResponse` containing the moderation results and usage
    /// statistics.
    pub async fn send_via(&self, client: &ZaiClient) -> crate::ZaiResult<ModerationResponse> {
        self.validate()?;
        let route = crate::client::routes::MODERATION_CHECK;
        let url = client.endpoints().resolve_route(route, &[])?;
        client
            .send_json::<_, ModerationResponse>(route.method(), url, &self.body)
            .await
    }
}
