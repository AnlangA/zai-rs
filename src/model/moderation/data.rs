//! Content-moderation request builder.

use super::models::*;
use crate::client::ZaiClient;

/// Validated content-moderation request.
///
/// Credentials and transport are supplied by the [`ZaiClient`] passed to
/// [`send_via`](Self::send_via).
pub struct Moderation {
    /// Moderation request body.
    body: ModerationRequest,
}

impl Moderation {
    /// Create a text-moderation request.
    ///
    /// The service accepts at most 2,000 Unicode scalar values.
    pub fn new_text(text: impl Into<String>) -> Self {
        let body = ModerationRequest::new_text(text);
        Self { body }
    }

    /// Create a multimedia-moderation request for `url`.
    pub fn new_multimedia(content_type: MediaType, url: impl Into<String>) -> Self {
        let body = ModerationRequest::new_multimedia(content_type, url);
        Self { body }
    }

    /// Create a moderation request for multiple structured items.
    pub fn new_items(items: Vec<ModerationItem>) -> Self {
        Self {
            body: ModerationRequest::new_items(items),
        }
    }

    /// Mutably borrow the request body.
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

    /// Validate and send the request through `client`.
    ///
    /// This method automatically validates the request before sending.
    ///
    pub async fn send_via(&self, client: &ZaiClient) -> crate::ZaiResult<ModerationResponse> {
        self.validate()?;
        let route = crate::client::routes::MODERATION_CHECK;
        client
            .operation(route)
            .send_json::<_, ModerationResponse>(&self.body)
            .await
    }
}
