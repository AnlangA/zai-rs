use super::super::traits::*;
use crate::client::http::HttpClient;
use std::marker::PhantomData;
pub struct AsyncChatGetRequest<N>
where
    N: ModelName + AsyncChat,
{
    pub key: String,
    url: String,
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
        let url = format!(
            "https://open.bigmodel.cn/api/paas/v4/async-result/{}",
            task_id
        );
        Self {
            key,
            url,
            _body: (),
            _marker: PhantomData,
        }
    }

    pub fn validate(&self) -> anyhow::Result<()> {
        if self.url.trim().is_empty() {
            return Err(anyhow::anyhow!("empty URL"));
        }
        Ok(())
    }

    pub async fn send(
        &self,
    ) -> anyhow::Result<crate::model::chat_base_response::ChatCompletionResponse> {
        self.validate()?;
        let resp = self.get().await?;
        let parsed = resp
            .json::<crate::model::chat_base_response::ChatCompletionResponse>()
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
}
