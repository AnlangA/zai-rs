use super::types::DocumentDetailResponse;
use crate::{ZaiResult, client::http::HttpClient};

/// Retrieve document detail by id
pub struct DocumentRetrieveRequest {
    /// Bearer API key
    pub key: String,
    url: String,
}

impl DocumentRetrieveRequest {
    /// Create a new request
    pub fn new(key: String, document_id: impl AsRef<str>) -> Self {
        let url = format!(
            "https://open.bigmodel.cn/api/llm-application/open/document/{}",
            document_id.as_ref()
        );
        Self { key, url }
    }

    /// Send GET request and parse typed response
    pub async fn send(&self) -> ZaiResult<DocumentDetailResponse> {
        let resp = self.get().await?;
        let parsed = resp.json::<DocumentDetailResponse>().await?;
        Ok(parsed)
    }
}

impl HttpClient for DocumentRetrieveRequest {
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
        &()
    }
}
