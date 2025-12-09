use url::Url;

use super::types::DocumentListResponse;
use crate::ZaiResult;
use crate::client::http::HttpClient;

/// Query parameters for listing documents under a knowledge base
#[derive(Debug, Clone, serde::Serialize, validator::Validate)]
pub struct DocumentListQuery {
    /// Knowledge base id (required)
    #[validate(length(min = 1))]
    pub knowledge_id: String,
    /// Page index (default 1)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(range(min = 1))]
    pub page: Option<u32>,
    /// Page size (default 10)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(range(min = 1))]
    pub size: Option<u32>,
    /// Document name filter
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(length(min = 1))]
    pub word: Option<String>,
}

impl DocumentListQuery {
    pub fn new(knowledge_id: impl Into<String>) -> Self {
        Self {
            knowledge_id: knowledge_id.into(),
            page: Some(1),
            size: Some(10),
            word: None,
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
    pub fn with_word(mut self, word: impl Into<String>) -> Self {
        self.word = Some(word.into());
        self
    }
}

/// Document list request (GET /llm-application/open/document)
pub struct DocumentListRequest {
    /// Bearer API key
    pub key: String,
    url: String,
    _body: (),
}

impl DocumentListRequest {
    pub fn new(key: String) -> Self {
        let url = "https://open.bigmodel.cn/api/llm-application/open/document".to_string();
        Self {
            key,
            url,
            _body: (),
        }
    }

    fn rebuild_url(&mut self, q: &DocumentListQuery) {
        let mut url =
            Url::parse("https://open.bigmodel.cn/api/llm-application/open/document").unwrap();
        {
            let mut pairs = url.query_pairs_mut();
            pairs.append_pair("knowledge_id", &q.knowledge_id);
            if let Some(page) = q.page.as_ref() {
                pairs.append_pair("page", &page.to_string());
            }
            if let Some(size) = q.size.as_ref() {
                pairs.append_pair("size", &size.to_string());
            }
            if let Some(word) = q.word.as_ref() {
                pairs.append_pair("word", word);
            }
        }
        self.url = url.to_string();
    }

    /// Apply query by rebuilding internal URL
    pub fn with_query(mut self, q: DocumentListQuery) -> Self {
        self.rebuild_url(&q);
        self
    }

    /// Validate query, rebuild URL, then send
    pub async fn send_with_query(
        mut self,
        q: &DocumentListQuery,
    ) -> ZaiResult<DocumentListResponse> {
        use validator::Validate;
        q.validate()?;
        self.rebuild_url(q);
        self.send().await
    }

    /// Send and parse typed response
    pub async fn send(&self) -> ZaiResult<DocumentListResponse> {
        let resp = self.get().await?;
        let parsed = resp.json::<DocumentListResponse>().await?;
        Ok(parsed)
    }
}

impl HttpClient for DocumentListRequest {
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
