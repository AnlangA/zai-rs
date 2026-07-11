use std::sync::Arc;

use super::{
    request::{EmbeddingBody, EmbeddingDimensions, EmbeddingInput, EmbeddingModel},
    response::EmbeddingResponse,
};
use crate::client::ZaiClient;
use crate::client::http::{HttpClientConfig, parse_typed_response, send_json_request};

/// Text Embedding request builder (P05: routes through [`ZaiClient`]).
///
/// Credentials and transport live on the `ZaiClient`, passed to
/// [`send_via`](Self::send_via).
pub struct EmbeddingRequest {
    body: EmbeddingBody,
}

impl EmbeddingRequest {
    /// Create a new embedding request for the given model and input.
    pub fn new(model: EmbeddingModel, input: EmbeddingInput) -> Self {
        Self {
            body: EmbeddingBody::new(model, input),
        }
    }

    /// Set the embedding vector dimensionality.
    pub fn with_dimensions(mut self, dims: EmbeddingDimensions) -> Self {
        self.body = self.body.with_dimensions(dims);
        self
    }

    /// Optional: validate constraints before sending.
    pub fn validate(&self) -> crate::ZaiResult<()> {
        self.body.validate_model_constraints().map_err(|e| {
            crate::client::error::ZaiError::ApiError {
                code: crate::client::error::codes::SDK_VALIDATION,
                message: format!("Validation error: {e:?}"),
            }
        })
    }

    /// Send via a [`ZaiClient`] and parse the typed response.
    pub async fn send_via(&self, client: &ZaiClient) -> crate::ZaiResult<EmbeddingResponse> {
        self.validate()?;
        let url = client
            .endpoints()
            .resolve(crate::client::ApiFamily::PaasV4, &["embeddings"])?;
        let config = transport_config_from_client(client);
        let resp = send_json_request(
            reqwest::Method::POST,
            url,
            client.secret().expose(),
            &self.body,
            Arc::new(config),
        )
        .await?;
        parse_typed_response::<EmbeddingResponse>(resp).await
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
