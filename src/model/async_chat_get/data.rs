use std::{marker::PhantomData, sync::Arc};

use super::super::traits::*;
use crate::client::{
    endpoints::{ApiBase, EndpointConfig, join_url, paths},
    http::{HttpClient, HttpClientConfig, parse_typed_response},
};
pub struct AsyncChatGetRequest<N>
where
    N: ModelName + AsyncChat,
{
    pub key: String,
    url: String,
    endpoint_config: EndpointConfig,
    api_base: ApiBase,
    task_id: String,
    http_config: Arc<HttpClientConfig>,
    // Empty body placeholder to satisfy HttpClient::Body
    _body: (),
    // Phantom placeholder to carry generic N for compile-time constraints
    _marker: PhantomData<N>,
}

impl<N> AsyncChatGetRequest<N>
where
    N: ModelName + AsyncChat,
{
    pub fn new(_model: N, task_id: String, key: String) -> Self {
        let endpoint_config = EndpointConfig::default();
        let api_base = ApiBase::PaasV4;
        let path = join_url(paths::ASYNC_RESULT, &task_id);
        let url = endpoint_config.url(&api_base, &path);
        Self {
            key,
            url,
            endpoint_config,
            api_base,
            task_id,
            http_config: Arc::new(HttpClientConfig::default()),
            _body: (),
            _marker: PhantomData,
        }
    }

    fn rebuild_url(&mut self) {
        let path = join_url(paths::ASYNC_RESULT, &self.task_id);
        self.url = self.endpoint_config.url(&self.api_base, &path);
    }

    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.api_base = ApiBase::Custom(base_url.into());
        self.rebuild_url();
        self
    }

    pub fn with_endpoint_config(mut self, endpoint_config: EndpointConfig) -> Self {
        self.endpoint_config = endpoint_config;
        self.rebuild_url();
        self
    }

    pub fn with_http_config(mut self, config: HttpClientConfig) -> Self {
        self.http_config = Arc::new(config);
        self
    }

    pub fn validate(&self) -> crate::ZaiResult<()> {
        if self.url.trim().is_empty() {
            return Err(crate::client::error::ZaiError::ApiError {
                code: 1200,
                message: "empty URL".to_string(),
            });
        }
        Ok(())
    }

    pub async fn send(
        &self,
    ) -> crate::ZaiResult<crate::model::chat_base_response::ChatCompletionResponse> {
        self.validate()?;

        let resp = self.get().await?;

        let parsed =
            parse_typed_response::<crate::model::chat_base_response::ChatCompletionResponse>(resp)
                .await?;

        Ok(parsed)
    }
}

impl<N> HttpClient for AsyncChatGetRequest<N>
where
    N: ModelName + AsyncChat,
{
    type Body = ();
    type ApiUrl = String;
    type ApiKey = String;

    fn api_url(&self) -> &Self::ApiUrl {
        &self.url
    }

    fn api_key(&self) -> &Self::ApiKey {
        &self.key
    }

    fn body(&self) -> &Self::Body {
        &self._body
    }

    fn http_config(&self) -> Arc<HttpClientConfig> {
        self.http_config.clone()
    }
}
