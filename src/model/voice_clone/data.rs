use std::sync::Arc;

use serde::Serialize;
use validator::Validate;

use super::{super::traits::*, request::VoiceCloneBody};
use crate::client::http::{HttpClientConfig, parse_typed_response, send_json_request};
use crate::client::v2::ZaiClient;

/// Voice clone request wrapper using JSON
///
/// Builder for the voice-clone endpoint. Construct with
/// [`VoiceCloneRequest::new`], tune with the `with_*` methods, then call
/// [`VoiceCloneRequest::send_via`].
///
/// **P05**: credentials and transport live on the [`ZaiClient`], passed to
/// [`send_via`](Self::send_via).
pub struct VoiceCloneRequest<N>
where
    N: ModelName + VoiceClone + Serialize,
{
    body: VoiceCloneBody<N>,
}

impl<N> VoiceCloneRequest<N>
where
    N: ModelName + VoiceClone + Serialize,
{
    /// Create a new voice clone request with required fields.
    ///
    /// **P05**: no longer takes an API key — the key is provided by the
    /// [`ZaiClient`] at send time.
    pub fn new(
        model: N,
        voice_name: impl Into<String>,
        input: impl Into<String>,
        file_id: impl Into<String>,
    ) -> Self {
        let body = VoiceCloneBody::new(model, voice_name, input, file_id);
        Self { body }
    }

    /// Borrow the underlying [`VoiceCloneBody`] mutably for advanced tweaks.
    pub fn body_mut(&mut self) -> &mut VoiceCloneBody<N> {
        &mut self.body
    }

    /// Optional: reference text of the example audio
    pub fn with_text(mut self, text: impl Into<String>) -> Self {
        self.body = self.body.with_text(text);
        self
    }

    /// Optional: client-provided request id
    pub fn with_request_id(mut self, request_id: impl Into<String>) -> Self {
        self.body = self.body.with_request_id(request_id);
        self
    }

    /// Validate the request body constraints before sending.
    pub fn validate(&self) -> crate::ZaiResult<()> {
        self.body
            .validate()
            .map_err(|e| crate::client::error::ZaiError::ApiError {
                code: crate::client::error::codes::SDK_VALIDATION,
                message: format!("Validation error: {e:?}"),
            })?;
        Ok(())
    }

    /// Submit the request via a [`ZaiClient`] and parse the typed voice-clone
    /// response.
    pub async fn send_via(
        &self,
        client: &ZaiClient,
    ) -> crate::ZaiResult<super::response::VoiceCloneResponse> {
        self.validate()?;
        let url = client
            .endpoints()
            .resolve(crate::client::v2::ApiFamily::PaasV4, &["voice", "clone"])?;
        let config = transport_config_from_client(client);
        let resp = send_json_request(
            reqwest::Method::POST,
            url,
            client.secret().expose(),
            &self.body,
            Arc::new(config),
        )
        .await?;
        parse_typed_response::<super::response::VoiceCloneResponse>(resp).await
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
