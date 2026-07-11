use std::sync::Arc;

use super::{
    request::{TokenizerBody, TokenizerMessage, TokenizerModel},
    response::TokenizerResponse,
};
use crate::client::ZaiClient;
use crate::client::http::{HttpClientConfig, parse_typed_response, send_json_request};

/// Text Tokenizer request client (JSON POST)
///
/// Builder for the tokenizer endpoint. Construct with
/// [`TokenizerRequest::new`], tune with the `with_*` methods, then call
/// [`TokenizerRequest::send_via`].
pub struct TokenizerRequest {
    body: TokenizerBody,
}

impl TokenizerRequest {
    /// Create a new tokenizer request for the given model and messages.
    pub fn new(model: TokenizerModel, messages: Vec<TokenizerMessage>) -> Self {
        let body = TokenizerBody::new(model, messages);
        Self { body }
    }

    /// Set the client-side request id.
    pub fn with_request_id(mut self, v: impl Into<String>) -> Self {
        self.body = self.body.with_request_id(v);
        self
    }
    /// Set the end-user id.
    pub fn with_user_id(mut self, v: impl Into<String>) -> Self {
        self.body = self.body.with_user_id(v);
        self
    }

    /// Optional: validate constraints before sending
    pub fn validate(&self) -> crate::ZaiResult<()> {
        if self.body.messages.is_empty() {
            return Err(crate::client::error::ZaiError::ApiError {
                code: crate::client::error::codes::SDK_VALIDATION,
                message: "messages must not be empty".to_string(),
            });
        }
        Ok(())
    }

    /// Send via a [`ZaiClient`] and parse the typed response.
    /// Automatically runs `validate()` before sending.
    pub async fn send_via(&self, client: &ZaiClient) -> crate::ZaiResult<TokenizerResponse> {
        self.validate()?;
        let url = client
            .endpoints()
            .resolve(crate::client::ApiFamily::PaasV4, &["tokenizer"])?;
        let config = transport_config_from_client(client);
        let resp = send_json_request(
            reqwest::Method::POST,
            url,
            client.secret().expose(),
            &self.body,
            Arc::new(config),
        )
        .await?;
        parse_typed_response::<TokenizerResponse>(resp).await
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
