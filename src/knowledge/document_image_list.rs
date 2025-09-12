use crate::client::http::HttpClient;
use super::types::DocumentImageListResponse;

/// Retrieve parsed image index-url mapping for a document (POST, no body)
pub struct DocumentImageListRequest {
    /// Bearer API key
    pub key: String,
    url: String,
}

impl DocumentImageListRequest {
    /// Create a new request with the target document id
    pub fn new(key: String, document_id: impl AsRef<str>) -> Self {
        let url = format!(
            "https://open.bigmodel.cn/api/llm-application/open/document/slice/image_list/{}",
            document_id.as_ref()
        );
        Self { key, url }
    }

    /// Send POST request and parse typed response
    pub async fn send(&self) -> anyhow::Result<DocumentImageListResponse> {
        let resp = self.post().await?;
        let parsed = resp.json::<DocumentImageListResponse>().await?;
        Ok(parsed)
    }
}

impl HttpClient for DocumentImageListRequest {
    type Body = (); // unused
    type ApiUrl = String;
    type ApiKey = String;

    fn api_url(&self) -> &Self::ApiUrl { &self.url }
    fn api_key(&self) -> &Self::ApiKey { &self.key }
    fn body(&self) -> &Self::Body { &() }

    // Override POST: send no body, only auth header
    fn post(&self) -> impl std::future::Future<Output = anyhow::Result<reqwest::Response>> + Send {
        let url = self.url.clone();
        let key = self.key.clone();
        async move {
            let resp = reqwest::Client::new()
                .post(url)
                .bearer_auth(key)
                .send()
                .await?;

            let status = resp.status();
            if status.is_success() { return Ok(resp); }

            // Standard error envelope {"error": { code, message }}
            let text = resp.text().await.unwrap_or_default();
            #[derive(serde::Deserialize)]
            struct ErrEnv { error: ErrObj }
            #[derive(serde::Deserialize)]
            struct ErrObj { code: serde_json::Value, message: String }
            if let Ok(parsed) = serde_json::from_str::<ErrEnv>(&text) {
                return Err(anyhow::anyhow!(
                    "HTTP {} {} | code={} | message={}",
                    status.as_u16(),
                    status.canonical_reason().unwrap_or(""),
                    parsed.error.code,
                    parsed.error.message
                ));
            }
            Err(anyhow::anyhow!(
                "HTTP {} {} | body={}",
                status.as_u16(),
                status.canonical_reason().unwrap_or(""),
                text
            ))
        }
    }
}

