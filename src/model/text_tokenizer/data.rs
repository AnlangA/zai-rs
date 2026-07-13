use super::{
    request::{TokenizerBody, TokenizerMessage, TokenizerModel},
    response::TokenizerResponse,
};
use crate::client::ZaiClient;

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

    /// Validate request constraints before sending.
    pub fn validate(&self) -> crate::ZaiResult<()> {
        self.body.validate()
    }

    /// Send via a [`ZaiClient`] and parse the typed response.
    /// Automatically runs `validate()` before sending.
    pub async fn send_via(&self, client: &ZaiClient) -> crate::ZaiResult<TokenizerResponse> {
        self.validate()?;
        let route = crate::client::routes::TOKENIZER_COUNT;
        client
            .operation(route)
            .send_json::<_, TokenizerResponse>(&self.body)
            .await
    }
}
