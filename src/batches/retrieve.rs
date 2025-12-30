use super::types::BatchItem;
use crate::{ZaiResult, client::http::HttpClient};

/// Retrieve a batch task by ID (GET /paas/v4/batches/{batch_id})
pub struct BatchesRetrieveRequest {
    /// Bearer API key
    pub key: String,
    /// Full URL with path parameter bound
    url: String,
    /// No body for GET
    _body: (),
}

impl BatchesRetrieveRequest {
    /// Create a new retrieve request with required path parameter `batch_id`.
    pub fn new(key: String, batch_id: impl AsRef<str>) -> Self {
        // Batch IDs are expected to be safe; if special chars appear, consider
        // encoding.
        let url = format!(
            "https://open.bigmodel.cn/api/paas/v4/batches/{}",
            batch_id.as_ref()
        );
        Self {
            key,
            url,
            _body: (),
        }
    }

    /// Send request and parse typed response as a single BatchItem
    pub async fn send(&self) -> ZaiResult<BatchesRetrieveResponse> {
        let resp: reqwest::Response = self.get().await?;
        let parsed = resp.json::<BatchesRetrieveResponse>().await?;
        Ok(parsed)
    }
}

impl HttpClient for BatchesRetrieveRequest {
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

/// Response type: a single Batch object
pub type BatchesRetrieveResponse = BatchItem;
