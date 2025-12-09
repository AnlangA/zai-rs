use super::request::{RerankBody, RerankModel};
use super::response::RerankResponse;
use crate::ZaiResult;
use crate::client::http::HttpClient;

/// Text Rerank request client (JSON POST)
pub struct RerankRequest {
    pub key: String,
    body: RerankBody,
}

impl RerankRequest {
    pub fn new(key: String, query: impl Into<String>, documents: Vec<String>) -> Self {
        let body = RerankBody::new(RerankModel::Rerank, query, documents);
        Self { key, body }
    }

    pub fn with_top_n(mut self, n: usize) -> Self {
        self.body = self.body.with_top_n(n);
        self
    }
    pub fn with_return_documents(mut self, v: bool) -> Self {
        self.body = self.body.with_return_documents(v);
        self
    }
    pub fn with_return_raw_scores(mut self, v: bool) -> Self {
        self.body = self.body.with_return_raw_scores(v);
        self
    }
    pub fn with_request_id(mut self, v: impl Into<String>) -> Self {
        self.body = self.body.with_request_id(v);
        self
    }
    pub fn with_user_id(mut self, v: impl Into<String>) -> Self {
        self.body = self.body.with_user_id(v);
        self
    }

    /// Optional: validate constraints before sending
    pub fn validate(&self) -> ZaiResult<()> {
        self.body
            .validate_constraints()
            .map_err(|e| crate::client::error::ZaiError::ApiError {
                code: 1200,
                message: format!("Validation error: {:?}", e),
            })
    }

    /// Send the request and parse typed response.
    /// Automatically runs `validate()` before sending.
    pub async fn send(&self) -> ZaiResult<RerankResponse> {
        self.validate()?;
        let resp: reqwest::Response = self.post().await?;
        let parsed = resp.json::<RerankResponse>().await?;
        Ok(parsed)
    }

    #[deprecated(note = "Use send() instead")]
    /// Deprecated: use `send()`.
    pub async fn execute(&self) -> ZaiResult<RerankResponse> {
        self.send().await
    }
}

impl HttpClient for RerankRequest {
    type Body = RerankBody;
    type ApiUrl = &'static str;
    type ApiKey = String;

    fn api_url(&self) -> &Self::ApiUrl {
        &"https://open.bigmodel.cn/api/paas/v4/rerank"
    }
    fn api_key(&self) -> &Self::ApiKey {
        &self.key
    }
    fn body(&self) -> &Self::Body {
        &self.body
    }
}
