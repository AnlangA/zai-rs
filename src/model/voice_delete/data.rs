use std::sync::Arc;

use validator::Validate;

use super::request::VoiceDeleteBody;
use crate::client::ZaiClient;
use crate::client::http::{HttpClientConfig, parse_typed_response, send_json_request};

/// Voice delete request using JSON body
///
/// Builder for the voice-delete endpoint. Construct with
/// [`VoiceDeleteRequest::new`], tune with the `with_*` methods, then call
/// [`VoiceDeleteRequest::send_via`].
///
/// **P05**: credentials and transport live on the [`ZaiClient`], passed to
/// [`send_via`](Self::send_via).
pub struct VoiceDeleteRequest {
    body: VoiceDeleteBody,
}

impl VoiceDeleteRequest {
    /// Create a new voice-delete request for the given voice id.
    ///
    /// **P05**: no longer takes an API key — the key is provided by the
    /// [`ZaiClient`] at send time.
    pub fn new(voice: impl Into<String>) -> Self {
        let body = VoiceDeleteBody::new(voice);
        Self { body }
    }

    /// Set the client-side request id.
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

    /// Submit the request via a [`ZaiClient`] and parse the typed voice-delete
    /// response.
    pub async fn send_via(
        &self,
        client: &ZaiClient,
    ) -> crate::ZaiResult<super::response::VoiceDeleteResponse> {
        self.validate()?;
        let url = client
            .endpoints()
            .resolve(crate::client::ApiFamily::PaasV4, &["voice", "delete"])?;
        let config = transport_config_from_client(client);
        let resp = send_json_request(
            reqwest::Method::POST,
            url,
            client.secret().expose(),
            &self.body,
            Arc::new(config),
        )
        .await?;
        parse_typed_response::<super::response::VoiceDeleteResponse>(resp).await
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
