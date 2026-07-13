use super::{
    request::{EmbeddingBody, EmbeddingDimensions, EmbeddingInput, EmbeddingModel},
    response::EmbeddingResponse,
};
use crate::client::ZaiClient;

/// Builder for generating text embeddings through a [`ZaiClient`].
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

    /// Validate input and model/dimension constraints before sending.
    pub fn validate(&self) -> crate::ZaiResult<()> {
        self.body.validate_model_constraints().map_err(|error| {
            crate::client::error::ZaiError::ApiError {
                code: crate::client::error::codes::SDK_VALIDATION,
                message: format!("embedding request validation failed: {}", error.code),
            }
        })
    }

    /// Send via a [`ZaiClient`] and parse the typed response.
    pub async fn send_via(&self, client: &ZaiClient) -> crate::ZaiResult<EmbeddingResponse> {
        self.validate()?;
        let route = crate::client::routes::EMBEDDINGS_CREATE;
        client
            .operation(route)
            .send_json::<_, EmbeddingResponse>(&self.body)
            .await
    }
}
