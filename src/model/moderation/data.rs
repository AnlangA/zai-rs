//! # Content Moderation API
//!
//! This module provides the content moderation client for analyzing text,
//! image, audio, and video content for safety risks.

use std::sync::Arc;

use super::models::*;
use crate::client::ZaiClient;
use crate::client::http::{HttpClientConfig, parse_typed_response, send_json_request};

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
    /// * `text` - The text content to moderate (max 2000 characters)
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
        let url = client
            .endpoints()
            .resolve(crate::client::ApiFamily::PaasV4, &["moderations"])?;
        let config = transport_config_from_client(client);
        let resp = send_json_request(
            reqwest::Method::POST,
            url,
            client.secret().expose(),
            &self.body,
            Arc::new(config),
        )
        .await?;
        parse_typed_response::<ModerationResponse>(resp).await
    }
}

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
