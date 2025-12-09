use url::Url;

use super::types::KnowledgeListResponse;
use crate::ZaiResult;
use crate::client::http::HttpClient;

/// Query parameters for knowledge list API
#[derive(Debug, Clone, Default, serde::Serialize, validator::Validate)]
pub struct KnowledgeListQuery {
    /// Page index starting from 1 (default 1)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(range(min = 1))]
    pub page: Option<u32>,
    /// Page size (default 10)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(range(min = 1))]
    pub size: Option<u32>,
}

impl KnowledgeListQuery {
    pub fn new() -> Self {
        Self {
            page: Some(1),
            size: Some(10),
        }
    }
    pub fn with_page(mut self, page: u32) -> Self {
        self.page = Some(page);
        self
    }
    pub fn with_size(mut self, size: u32) -> Self {
        self.size = Some(size);
        self
    }
}

/// Knowledge list request (GET /llm-application/open/knowledge)
pub struct KnowledgeListRequest {
    /// Bearer API key
    pub key: String,
    url: String,
    _body: (),
}

impl KnowledgeListRequest {
    pub fn new(key: String) -> Self {
        let url = "https://open.bigmodel.cn/api/llm-application/open/knowledge".to_string();
        Self {
            key,
            url,
            _body: (),
        }
    }

    fn rebuild_url(&mut self, q: &KnowledgeListQuery) {
        let mut url =
            Url::parse("https://open.bigmodel.cn/api/llm-application/open/knowledge").unwrap();
        {
            let mut pairs = url.query_pairs_mut();
            if let Some(page) = q.page.as_ref() {
                pairs.append_pair("page", &page.to_string());
            }
            if let Some(size) = q.size.as_ref() {
                pairs.append_pair("size", &size.to_string());
            }
        }
        self.url = url.to_string();
    }

    /// Apply query by rebuilding internal URL
    pub fn with_query(mut self, q: KnowledgeListQuery) -> Self {
        self.rebuild_url(&q);
        self
    }

    /// Send request and parse typed response
    pub async fn send(&self) -> ZaiResult<KnowledgeListResponse> {
        let resp = self.get().await?;
        let parsed = resp.json::<KnowledgeListResponse>().await?;
        Ok(parsed)
    }

    /// Validate query, rebuild URL then send
    pub async fn send_with_query(
        mut self,
        q: &KnowledgeListQuery,
    ) -> ZaiResult<KnowledgeListResponse> {
        use validator::Validate;
        q.validate()?;
        self.rebuild_url(q);
        self.send().await
    }
}

impl HttpClient for KnowledgeListRequest {
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
