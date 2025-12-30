use url::Url;

use super::request::FileListQuery;
use crate::{ZaiResult, client::http::HttpClient};

/// Files list request (GET /paas/v4/files)
///
/// Builds query parameters from `FileListQuery` and performs an authenticated
/// GET.
pub struct FileListRequest {
    pub key: String,
    url: String,
    _body: (),
}

impl FileListRequest {
    pub fn new(key: String) -> Self {
        let url = "https://open.bigmodel.cn/api/paas/v4/files".to_string();
        Self {
            key,
            url,
            _body: (),
        }
    }

    fn rebuild_url(&mut self, q: &FileListQuery) {
        let mut url = Url::parse("https://open.bigmodel.cn/api/paas/v4/files").unwrap();
        {
            let mut pairs = url.query_pairs_mut();
            if let Some(after) = q.after.as_ref() {
                pairs.append_pair("after", after);
            }
            if let Some(purpose) = q.purpose.as_ref() {
                pairs.append_pair("purpose", purpose.as_str());
            }
            if let Some(order) = q.order.as_ref() {
                pairs.append_pair("order", order.as_str());
            }
            if let Some(limit) = q.limit.as_ref() {
                pairs.append_pair("limit", &limit.to_string());
            }
        }
        self.url = url.to_string();
    }

    pub fn with_query(mut self, q: FileListQuery) -> Self {
        self.rebuild_url(&q);
        self
    }
    /// Send request and parse typed response.
    pub async fn send(&self) -> ZaiResult<super::response::FileListResponse> {
        let resp = self.get().await?;
        let parsed = resp.json::<super::response::FileListResponse>().await?;
        Ok(parsed)
    }

    /// Validate query, rebuild URL and send in one call.
    pub async fn send_with_query(
        mut self,
        q: &super::request::FileListQuery,
    ) -> ZaiResult<super::response::FileListResponse> {
        use validator::Validate;
        q.validate()?;
        self.rebuild_url(q);
        self.send().await
    }
}

impl HttpClient for FileListRequest {
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
