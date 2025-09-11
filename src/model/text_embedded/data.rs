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

    /// Convenience method to execute request and parse typed response
    pub async fn execute(&self) -> anyhow::Result<EmbeddingResponse> {
        let resp: reqwest::Response = self.post().await?;
        let parsed = resp.json::<EmbeddingResponse>().await?;
        Ok(parsed)
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

