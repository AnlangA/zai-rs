use crate::client::http::HttpClient;
use super::types::KnowledgeDetailResponse;

/// Knowledge detail request (GET /llm-application/open/knowledge/{id})
pub struct KnowledgeRetrieveRequest {
    /// Bearer API key
    pub key: String,
    url: String,
    _body: (),
}

impl KnowledgeRetrieveRequest {
    /// Build a retrieve request with id
    pub fn new(key: String, id: impl AsRef<str>) -> Self {
        let url = format!(
            "https://open.bigmodel.cn/api/llm-application/open/knowledge/{}",
            id.as_ref()
        );
        Self { key, url, _body: () }
    }

    /// Send and parse typed response
    pub async fn send(&self) -> anyhow::Result<KnowledgeDetailResponse> {
        let resp = self.get().await?;
        let parsed = resp.json::<KnowledgeDetailResponse>().await?;
        Ok(parsed)
    }
}

impl HttpClient for KnowledgeRetrieveRequest {
    type Body = ();
    type ApiUrl = String;
    type ApiKey = String;

    fn api_url(&self) -> &Self::ApiUrl { &self.url }
    fn api_key(&self) -> &Self::ApiKey { &self.key }
    fn body(&self) -> &Self::Body { &self._body }
}

/// Alias for symmetry with other modules
pub type KnowledgeRetrieveResponse = KnowledgeDetailResponse;

