use super::types::KnowledgeCapacityResponse;
use crate::client::http::HttpClient;

/// Knowledge capacity request (GET /llm-application/open/knowledge/capacity)
pub struct KnowledgeCapacityRequest {
    /// Bearer API key
    pub key: String,
    url: String,
    _body: (),
}

impl KnowledgeCapacityRequest {
    /// Build a capacity request
    pub fn new(key: String) -> Self {
        let url =
            "https://open.bigmodel.cn/api/llm-application/open/knowledge/capacity".to_string();
        Self {
            key,
            url,
            _body: (),
        }
    }

    /// Send and parse typed response
    pub async fn send(&self) -> crate::ZaiResult<KnowledgeCapacityResponse> {
        let resp = self.get().await?;

        let parsed = resp.json::<KnowledgeCapacityResponse>().await?;

        Ok(parsed)
    }
}

impl HttpClient for KnowledgeCapacityRequest {
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
