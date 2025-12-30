use super::{
    request::{TokenizerBody, TokenizerMessage, TokenizerModel},
    response::TokenizerResponse,
};
use crate::client::http::HttpClient;

/// Text Tokenizer request client (JSON POST)
pub struct TokenizerRequest {
    pub key: String,
    body: TokenizerBody,
}

impl TokenizerRequest {
    pub fn new(key: String, model: TokenizerModel, messages: Vec<TokenizerMessage>) -> Self {
        let body = TokenizerBody::new(model, messages);
        Self { key, body }
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
    pub fn validate(&self) -> crate::ZaiResult<()> {
        if self.body.messages.is_empty() {
            return Err(crate::client::error::ZaiError::ApiError {
                code: 1200,
                message: "messages must not be empty".to_string(),
            });
        }
        Ok(())
    }

    /// Send the request and parse typed response.
    /// Automatically runs `validate()` before sending.
    pub async fn send(&self) -> crate::ZaiResult<TokenizerResponse> {
        self.validate()?;
        let resp: reqwest::Response = self.post().await?;
        let parsed = resp.json::<TokenizerResponse>().await?;
        Ok(parsed)
    }

    #[deprecated(note = "Use send() instead")]
    /// Deprecated: use `send()`.
    pub async fn execute(&self) -> crate::ZaiResult<TokenizerResponse> {
        self.send().await
    }
}

impl HttpClient for TokenizerRequest {
    type Body = TokenizerBody;
    type ApiUrl = &'static str;
    type ApiKey = String;

    fn api_url(&self) -> &Self::ApiUrl {
        &"https://open.bigmodel.cn/api/paas/v4/tokenizer"
    }
    fn api_key(&self) -> &Self::ApiKey {
        &self.key
    }
    fn body(&self) -> &Self::Body {
        &self.body
    }
}
