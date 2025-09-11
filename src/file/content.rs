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
