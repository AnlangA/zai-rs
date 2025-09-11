use super::request::{EmbeddingBody, EmbeddingDimensions, EmbeddingInput, EmbeddingModel};
use super::response::EmbeddingResponse;
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
    pub fn validate(&self) -> Result<(), validator::ValidationError> {
        self.body.validate_model_constraints()
    }

    /// Send the request and parse typed response.
    /// Automatically runs `validate()` before sending.
    pub async fn send(&self) -> anyhow::Result<EmbeddingResponse> {
        if let Err(e) = self.validate() {
            return Err(anyhow::anyhow!("validation failed: {}", e));
        }
        let resp: reqwest::Response = self.post().await?;
        let parsed = resp.json::<EmbeddingResponse>().await?;
        Ok(parsed)
    }

    #[deprecated(note = "Use send() instead")]
    /// Deprecated: use `send()`.
    pub async fn execute(&self) -> anyhow::Result<EmbeddingResponse> {
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

