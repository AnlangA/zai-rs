use crate::client::http::HttpClient;
use url::Url;

use super::request::FileListQuery;

/// Files list request (GET)
pub struct FileListRequest {
    pub key: String,
    url: String,
    _body: (),
}

impl FileListRequest {
    pub fn new(key: String) -> Self {
        let url = "https://open.bigmodel.cn/api/paas/v4/files".to_string();
        Self { key, url, _body: () }
    }

    fn rebuild_url(&mut self, q: &FileListQuery) {
        let mut url = Url::parse("https://open.bigmodel.cn/api/paas/v4/files").unwrap();
        {
            let mut pairs = url.query_pairs_mut();
            if let Some(after) = q.after.as_ref() { pairs.append_pair("after", after); }
            if let Some(purpose) = q.purpose.as_ref() { pairs.append_pair("purpose", purpose.as_str()); }
            if let Some(order) = q.order.as_ref() { pairs.append_pair("order", order.as_str()); }
            if let Some(limit) = q.limit.as_ref() { pairs.append_pair("limit", &limit.to_string()); }
        }
        self.url = url.to_string();
    }

    pub fn with_query(mut self, q: FileListQuery) -> Self {
        self.rebuild_url(&q);
        self
    }
}

impl HttpClient for FileListRequest {
    type Body = ();
    type ApiUrl = String;
    type ApiKey = String;

    fn api_url(&self) -> &Self::ApiUrl { &self.url }
    fn api_key(&self) -> &Self::ApiKey { &self.key }
    fn body(&self) -> &Self::Body { &self._body }
}



use std::path::PathBuf;

use super::request::FilePurpose;

/// File upload request (multipart/form-data)
pub struct FileUploadRequest {
    pub key: String,
    purpose: FilePurpose,
    file_path: PathBuf,
    file_name: Option<String>,
    content_type: Option<String>,
}

impl FileUploadRequest {
    pub fn new(key: String, purpose: FilePurpose, file_path: impl Into<PathBuf>) -> Self {
        Self {
            key,
            purpose,
            file_path: file_path.into(),
            file_name: None,
            content_type: None,
        }
    }

    pub fn with_file_name(mut self, name: impl Into<String>) -> Self {
        self.file_name = Some(name.into());
        self
    }

    pub fn with_content_type(mut self, ct: impl Into<String>) -> Self {
        self.content_type = Some(ct.into());
        self
    }
}

impl crate::client::http::HttpClient for FileUploadRequest {
    type Body = (); // not used
    type ApiUrl = &'static str;
    type ApiKey = String;

    fn api_url(&self) -> &Self::ApiUrl { &"https://open.bigmodel.cn/api/paas/v4/files" }
    fn api_key(&self) -> &Self::ApiKey { &self.key }
    fn body(&self) -> &Self::Body { &() }

    fn post(&self) -> impl std::future::Future<Output = anyhow::Result<reqwest::Response>> + Send {
        let url: String = "https://open.bigmodel.cn/api/paas/v4/files".to_string();
        let key: String = self.key.clone();
        let purpose = self.purpose.clone();
        let path = self.file_path.clone();
        let file_name = self.file_name.clone();
        let content_type = self.content_type.clone();
        async move {
            // Build multipart form
            let mut form = reqwest::multipart::Form::new().text("purpose", purpose.as_str().to_string());

            let fname = file_name
                .or_else(|| path.file_name().and_then(|s| s.to_str()).map(|s| s.to_string()))
                .unwrap_or_else(|| "upload.bin".to_string());

            let mut part = reqwest::multipart::Part::bytes(std::fs::read(&path)?).file_name(fname);
            if let Some(ct) = content_type {
                part = part.mime_str(&ct).map_err(|e| anyhow::anyhow!("invalid content-type: {}", e))?;
            }
            form = form.part("file", part);

            let resp = reqwest::Client::new()
                .post(url)
                .bearer_auth(key)
                .multipart(form)
                .send()
                .await?;

            let status = resp.status();
            if status.is_success() {
                return Ok(resp);
            }

            // Error parsing compatible with {"error": { code, message }}
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
            } else {
                return Err(anyhow::anyhow!(
                    "HTTP {} {} | body={}",
                    status.as_u16(),
                    status.canonical_reason().unwrap_or(""),
                    text
                ));
            }
        }
    }
}


/// File delete request (DELETE /files/{file_id})
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
        Self { key, url, _body: () }
    }

    /// Perform HTTP DELETE with error handling compatible with {"error":{...}}
    pub fn delete(&self) -> impl std::future::Future<Output = anyhow::Result<reqwest::Response>> + Send {
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
            } else {
                return Err(anyhow::anyhow!(
                    "HTTP {} {} | body={}",
                    status.as_u16(),
                    status.canonical_reason().unwrap_or(""),
                    text
                ));
            }
        }
    }
}

impl crate::client::http::HttpClient for FileDeleteRequest {
    type Body = ();
    type ApiUrl = String;
    type ApiKey = String;

    fn api_url(&self) -> &Self::ApiUrl { &self.url }
    fn api_key(&self) -> &Self::ApiKey { &self.key }
    fn body(&self) -> &Self::Body { &self._body }
}


/// File content request (GET /files/{file_id}/content)
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
        Self { key, url, _body: () }
    }
}

impl crate::client::http::HttpClient for FileContentRequest {
    type Body = ();
    type ApiUrl = String;
    type ApiKey = String;

    fn api_url(&self) -> &Self::ApiUrl { &self.url }
    fn api_key(&self) -> &Self::ApiKey { &self.key }
    fn body(&self) -> &Self::Body { &self._body }
}
