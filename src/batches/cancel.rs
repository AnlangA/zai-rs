use serde::{Deserialize, Serialize};

use super::types::BatchItem;
use crate::ZaiResult;
use crate::client::http::HttpClient;

/// Empty body for cancel API (serializes to `{}`)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CancelBatchBody {}

/// Cancel a running batch (POST /paas/v4/batches/{batch_id}/cancel)
pub struct CancelBatchRequest {
    /// Bearer API key
    pub key: String,
    /// Full URL including path parameter
    url: String,
    /// Empty JSON body
    body: CancelBatchBody,
}

impl CancelBatchRequest {
    /// Create a new cancel request for the given batch_id
    pub fn new(key: String, batch_id: impl AsRef<str>) -> Self {
        let url = format!(
            "https://open.bigmodel.cn/api/paas/v4/batches/{}/cancel",
            batch_id.as_ref()
        );
        Self {
            key,
            url,
            body: CancelBatchBody::default(),
        }
    }

    /// Send the request and parse typed response
    pub async fn send(&self) -> ZaiResult<CancelBatchResponse> {
        let resp: reqwest::Response = self.post().await?;
        let parsed = resp.json::<CancelBatchResponse>().await?;
        Ok(parsed)
    }
}

impl HttpClient for CancelBatchRequest {
    type Body = CancelBatchBody;
    type ApiUrl = String;
    type ApiKey = String;

    fn api_url(&self) -> &Self::ApiUrl {
        &self.url
    }
    fn api_key(&self) -> &Self::ApiKey {
        &self.key
    }
    fn body(&self) -> &Self::Body {
        &self.body
    }
}

/// Response type: a single Batch object
pub type CancelBatchResponse = BatchItem;
