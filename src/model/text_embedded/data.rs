use super::{
    request::{EmbeddingBody, EmbeddingDimensions, EmbeddingInput, EmbeddingModel},
    response::EmbeddingResponse,
};
use crate::client::http::HttpClient;

/// Text Embedding request client (JSON POST)
pub struct EmbeddingRequest {
    pub key: String,
    body: EmbeddingBody,
}

impl EmbeddingRequest {
    pub fn new(key: String, model: EmbeddingModel, input: EmbeddingInput) -> Self {
        let body = EmbeddingBody::new(model, input);
        Self { key, body }
    }

    pub fn with_dimensions(mut self, dims: EmbeddingDimensions) -> Self {
        self.body = self.body.with_dimensions(dims);
        self
    }

    /// Optional: validate constraints before sending
    pub fn validate(&self) -> crate::ZaiResult<()> {
        self.body.validate_model_constraints().map_err(|e| {
            crate::client::error::ZaiError::ApiError {
                code: 1200,
                message: format!("Validation error: {:?}", e),
            }
        })
    }

    /// Send the request and parse typed response.
    /// Automatically runs `validate()` before sending.
    pub async fn send(&self) -> crate::ZaiResult<EmbeddingResponse> {
        if let Err(e) = self.validate() {
            return Err(crate::client::error::ZaiError::ApiError {
                code: 1200,
                message: format!("validation failed: {}", e),
            });
        }
        let resp: reqwest::Response = self.post().await?;
        let parsed = resp.json::<EmbeddingResponse>().await?;
        Ok(parsed)
    }

    #[deprecated(note = "Use send() instead")]
    /// Deprecated: use `send()`.
    pub async fn execute(&self) -> crate::ZaiResult<EmbeddingResponse> {
        self.send().await
    }
}

impl HttpClient for EmbeddingRequest {
    type Body = EmbeddingBody;
    type ApiUrl = &'static str;
    type ApiKey = String;

    fn api_url(&self) -> &Self::ApiUrl {
        &"https://open.bigmodel.cn/api/paas/v4/embeddings"
    }
    fn api_key(&self) -> &Self::ApiKey {
        &self.key
    }
    fn body(&self) -> &Self::Body {
        &self.body
    }
}
