use crate::client::http::HttpClient;

/// Document delete request (DELETE /llm-application/open/document/{id})
pub struct DocumentDeleteRequest {
    /// Bearer API key
    pub key: String,
    url: String,
    _body: (),
}

impl DocumentDeleteRequest {
    /// Build a delete request with target document id
    pub fn new(key: String, id: impl AsRef<str>) -> Self {
        let url = format!(
            "https://open.bigmodel.cn/api/llm-application/open/document/{}",
            id.as_ref()
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
            }

            Err(crate::client::error::ZaiError::from_api_response(
                status.as_u16(),
                0,
                text,
            ))
        }
    }

    /// Send delete request and parse typed response

    pub async fn send(&self) -> crate::ZaiResult<DocumentDeleteResponse> {
        let resp = self.delete().await?;

        let parsed = resp.json::<DocumentDeleteResponse>().await?;

        Ok(parsed)
    }
}

impl HttpClient for DocumentDeleteRequest {
    type Body = (); // unused
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

/// Delete response envelope without data
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, validator::Validate)]
pub struct DocumentDeleteResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<u64>,
}
