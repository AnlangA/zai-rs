use super::request::VoiceListQuery;
use crate::client::http::HttpClient;
use url::Url;

/// GET voice list request
pub struct VoiceListRequest {
    pub key: String,
    url: String,
    // Empty body placeholder to satisfy HttpClient::Body
    _body: (),
}

impl VoiceListRequest {
    pub fn new(key: String) -> Self {
        let url = "https://open.bigmodel.cn/api/paas/v4/voice/list".to_string();
        Self {
            key,
            url,
            _body: (),
        }
    }

    fn rebuild_url(&mut self, q: &VoiceListQuery) {
        let mut url = Url::parse("https://open.bigmodel.cn/api/paas/v4/voice/list").unwrap();
        {
            let mut pairs = url.query_pairs_mut();
            if let Some(ref n) = q.voice_name {
                pairs.append_pair("voiceName", n);
            }
            if let Some(ref t) = q.voice_type {
                pairs.append_pair("voiceType", t.as_str());
            }
        }
        self.url = url.to_string();
    }

    pub fn validate(&self) -> anyhow::Result<()> {
        // No required params; URL already built. Optionally, validate query formats here.
        Ok(())
    }

    pub async fn send(&self) -> anyhow::Result<super::response::VoiceListResponse> {
        self.validate()?;
        let resp = self.get().await?;
        let parsed = resp.json::<super::response::VoiceListResponse>().await?;
        Ok(parsed)
    }

    pub fn with_query(mut self, q: VoiceListQuery) -> Self {
        self.rebuild_url(&q);
        self
    }
}

impl HttpClient for VoiceListRequest {
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
