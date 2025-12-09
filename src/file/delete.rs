use crate::client::http::HttpClient;

/// File delete request (DELETE /paas/v4/files/{file_id})
pub struct FileDeleteRequest {
    pub key: String,
    url: String,
    _body: (),
}

impl FileDeleteRequest {
    pub fn new(key: String, file_id: impl Into<String>) -> Self {
        let url = format!(
            "https://open.bigmodel.cn/api/paas/v4/files/{}",
            file_id.into()
        );
        Self {
            key,
            url,
            _body: (),
        }
    }

    pub fn delete(
        &self,
    ) -> impl std::future::Future<Output = crate::ZaiResult<reqwest::Response>> + Send {
        let url = self.url.clone();
        let key = self.key.clone();
        async move {
            let resp = reqwest::Client::new()
                .delete(url)
                .bearer_auth(key)
                .send()
                .await?;

            let status = resp.status();
            if status.is_success() {
                return Ok(resp);
            }
            let text = resp.text().await.unwrap_or_default();
            #[derive(serde::Deserialize)]
            struct ErrEnv {
                error: ErrObj,
            }
            #[derive(serde::Deserialize)]
            struct ErrObj {
                _code: serde_json::Value,
                message: String,
            }

            if let Ok(parsed) = serde_json::from_str::<ErrEnv>(&text) {
                return Err(crate::client::error::ZaiError::from_api_response(
                    status.as_u16(),
                    0,
                    parsed.error.message,
                ));
            } else {
                return Err(crate::client::error::ZaiError::from_api_response(
                    status.as_u16(),
                    0,
                    text,
                ));
            }
        }
    }

    /// Send delete request and parse typed response.

    pub async fn send(&self) -> crate::ZaiResult<super::response::FileDeleteResponse> {
        let resp = self.delete().await?;
        let parsed = resp.json::<super::response::FileDeleteResponse>().await?;
        Ok(parsed)
    }
}

impl HttpClient for FileDeleteRequest {
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
