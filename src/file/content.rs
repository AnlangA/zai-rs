use crate::client::http::HttpClient;

/// File content request (GET /paas/v4/files/{file_id}/content)
pub struct FileContentRequest {
    pub key: String,
    url: String,
    _body: (),
}

impl FileContentRequest {
    pub fn new(key: String, file_id: impl Into<String>) -> Self {
        let url = format!(
            "https://open.bigmodel.cn/api/paas/v4/files/{}/content",
            file_id.into()
        );
        Self {
            key,
            url,
            _body: (),
        }
    }

    /// Send the request and return raw bytes of the file content.
    pub async fn send(&self) -> anyhow::Result<Vec<u8>> {
        let resp: reqwest::Response = self.get().await?;
        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "HTTP {} {} | body={}",
                status.as_u16(),
                status.canonical_reason().unwrap_or(""),
                text
            ));
        }
        let bytes = resp.bytes().await?;
        Ok(bytes.to_vec())
    }

    /// Send the request and save content to the given path.
    /// It will create parent directories if missing.
    /// Returns the number of bytes written.
    pub async fn send_to<P: AsRef<std::path::Path>>(&self, path: P) -> anyhow::Result<usize> {
        let bytes = self.send().await?;
        let p = path.as_ref();
        if let Some(parent) = p.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        use std::io::Write;
        let mut f = std::fs::File::create(p)?;
        f.write_all(&bytes)?;
        Ok(bytes.len())
    }
}

impl HttpClient for FileContentRequest {
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
