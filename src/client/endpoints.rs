//! Endpoint registry for Zhipu AI / BigModel API families.
//!
//! Keep endpoint bases and paths centralized so product modules describe
//! behavior instead of re-encoding transport URLs.

/// Official default base for general PAAS v4 APIs.
pub const PAAS_V4_BASE: &str = "https://open.bigmodel.cn/api/paas/v4";
/// Official default base for Coding Plan PAAS v4 APIs.
pub const CODING_PAAS_V4_BASE: &str = "https://open.bigmodel.cn/api/coding/paas/v4";
/// Official default base for knowledge-base APIs.
pub const LLM_APPLICATION_BASE: &str = "https://open.bigmodel.cn/api/llm-application/open";
/// Official default base for realtime APIs.
///
/// Verified against the official GLM-Realtime protocol
/// (<https://github.com/MetaGLM/glm-realtime-sdk/blob/main/GLM-Realtime-doc-for-llm.md>
/// and <https://docs.bigmodel.cn/cn/asyncapi/realtime>): the realtime endpoint
/// lives under `/api/paas/v4/realtime`, same v4 family as the REST APIs.
pub const REALTIME_BASE: &str = "wss://open.bigmodel.cn/api/paas/v4/realtime";

/// Official default base for the monitor / usage-statistics APIs.
///
/// The GLM Coding Plan exposes a usage/quota query endpoint under
/// `/api/monitor/usage/quota/limit` (verified via the official
/// `glm-plan-usage` plugin at
/// <https://docs.bigmodel.cn/cn/coding-plan/extension/usage-query-plugin> and
/// the community CLI <https://github.com/JinHanAI/coding-plan-monitor>). It
/// returns the per-5-hour and weekly quota consumption/remaining amounts.
pub const MONITOR_BASE: &str = "https://open.bigmodel.cn/api/monitor";

/// API family selector used by [`EndpointConfig`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApiBase {
    PaasV4,
    CodingPaasV4,
    LlmApplication,
    Realtime,
    /// Monitor / usage-statistics endpoints (Coding Plan quota query).
    Monitor,
    Custom(String),
}

/// Runtime-configurable API bases.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EndpointConfig {
    pub paas_v4_base: String,
    pub coding_paas_v4_base: String,
    pub llm_application_base: String,
    pub realtime_base: String,
    /// Monitor / usage-statistics base URL.
    pub monitor_base: String,
}

impl Default for EndpointConfig {
    fn default() -> Self {
        Self {
            paas_v4_base: PAAS_V4_BASE.to_string(),
            coding_paas_v4_base: CODING_PAAS_V4_BASE.to_string(),
            llm_application_base: LLM_APPLICATION_BASE.to_string(),
            realtime_base: REALTIME_BASE.to_string(),
            monitor_base: MONITOR_BASE.to_string(),
        }
    }
}

impl EndpointConfig {
    pub fn with_paas_v4_base(mut self, base: impl Into<String>) -> Self {
        self.paas_v4_base = base.into();
        self
    }

    pub fn with_coding_paas_v4_base(mut self, base: impl Into<String>) -> Self {
        self.coding_paas_v4_base = base.into();
        self
    }

    pub fn with_llm_application_base(mut self, base: impl Into<String>) -> Self {
        self.llm_application_base = base.into();
        self
    }

    pub fn with_realtime_base(mut self, base: impl Into<String>) -> Self {
        self.realtime_base = base.into();
        self
    }

    /// Override the monitor / usage-statistics base URL.
    pub fn with_monitor_base(mut self, base: impl Into<String>) -> Self {
        self.monitor_base = base.into();
        self
    }

    pub fn base<'a>(&'a self, api_base: &'a ApiBase) -> &'a str {
        match api_base {
            ApiBase::PaasV4 => &self.paas_v4_base,
            ApiBase::CodingPaasV4 => &self.coding_paas_v4_base,
            ApiBase::LlmApplication => &self.llm_application_base,
            ApiBase::Realtime => &self.realtime_base,
            ApiBase::Monitor => &self.monitor_base,
            ApiBase::Custom(base) => base,
        }
    }

    pub fn url(&self, api_base: &ApiBase, path: &str) -> String {
        join_url(self.base(api_base), path)
    }
}

/// Join a base URL and an endpoint path without duplicating slashes.
pub fn join_url(base: &str, path: &str) -> String {
    let base = base.trim_end_matches('/');
    let path = path.trim_start_matches('/');
    if path.is_empty() {
        base.to_string()
    } else {
        format!("{}/{}", base, path)
    }
}

/// Build a URL by appending query parameters to a base.
///
/// Centralizes query-string construction across list-style endpoints so every
/// module uses identical (and panic-free) percent-encoding via `url::Url`. The
/// iterator is generic so callers may pass `Vec<(&str, String)>`, arrays, etc.
///
/// If `base_url` is not a parseable absolute URL (a malformed user-supplied
/// `ApiBase::Custom`), it falls back to naive string concatenation rather than
/// panicking — a genuinely broken base will surface as a network error at
/// request time. This keeps the fluent `with_*` builder API infallible.
pub fn build_query<K, V, I>(base_url: &str, params: I) -> String
where
    K: AsRef<str>,
    V: AsRef<str>,
    I: IntoIterator<Item = (K, V)>,
{
    let collected: Vec<(K, V)> = params.into_iter().collect();
    if let Ok(mut url) = url::Url::parse(base_url) {
        if !collected.is_empty() {
            url.query_pairs_mut()
                .extend_pairs(collected.iter().map(|(k, v)| (k.as_ref(), v.as_ref())));
        }
        return url.to_string();
    }

    // Fallback for non-parseable bases (malformed custom URL): best-effort join.
    if collected.is_empty() {
        return base_url.to_string();
    }
    let query = collected
        .iter()
        .map(|(k, v)| format!("{}={}", k.as_ref(), v.as_ref()))
        .collect::<Vec<_>>()
        .join("&");
    format!("{base_url}?{query}")
}

pub mod paths {
    pub const CHAT_COMPLETIONS: &str = "chat/completions";
    pub const ASYNC_CHAT_COMPLETIONS: &str = "async/chat/completions";
    pub const ASYNC_RESULT: &str = "async-result";
    pub const EMBEDDINGS: &str = "embeddings";
    pub const RERANK: &str = "rerank";
    pub const TOKENIZER: &str = "tokenizer";
    pub const MODERATIONS: &str = "moderations";
    pub const IMAGES_GENERATIONS: &str = "images/generations";
    pub const VIDEOS_GENERATIONS: &str = "videos/generations";
    pub const AUDIO_TRANSCRIPTIONS: &str = "audio/transcriptions";
    pub const AUDIO_SPEECH: &str = "audio/speech";
    pub const VOICE_CLONE: &str = "voice/clone";
    pub const VOICE_LIST: &str = "voice/list";
    pub const VOICE_DELETE: &str = "voice/delete";
    pub const FILES: &str = "files";
    pub const FILES_OCR: &str = "files/ocr";
    pub const FILE_PARSER_CREATE: &str = "files/parser/create";
    pub const FILE_PARSER_RESULT: &str = "files/parser/result";
    pub const WEB_SEARCH: &str = "web_search";
    pub const BATCHES: &str = "batches";
    pub const AGENTS: &str = "agents";

    pub const KNOWLEDGE: &str = "knowledge";
    pub const KNOWLEDGE_CAPACITY: &str = "knowledge/capacity";
    pub const DOCUMENT: &str = "document";
    pub const DOCUMENT_UPLOAD_URL: &str = "document/upload_url";
    pub const DOCUMENT_UPLOAD_DOCUMENT: &str = "document/upload_document";
    pub const DOCUMENT_EMBEDDING: &str = "document/embedding";
    pub const DOCUMENT_SLICE_IMAGE_LIST: &str = "document/slice/image_list";

    /// GLM Coding Plan quota/usage query (GET, under the monitor base).
    /// Returns per-5-hour and weekly quota consumption + remaining amounts.
    pub const MONITOR_USAGE_QUOTA_LIMIT: &str = "usage/quota/limit";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn joins_base_and_path_without_double_slashes() {
        assert_eq!(
            join_url("https://open.bigmodel.cn/api/paas/v4/", "/chat/completions"),
            "https://open.bigmodel.cn/api/paas/v4/chat/completions"
        );
    }

    #[test]
    fn default_config_uses_official_bases() {
        let config = EndpointConfig::default();
        assert_eq!(config.paas_v4_base, PAAS_V4_BASE);
        assert_eq!(config.coding_paas_v4_base, CODING_PAAS_V4_BASE);
        assert_eq!(config.llm_application_base, LLM_APPLICATION_BASE);
    }

    #[test]
    fn realtime_base_matches_official_endpoint() {
        // Verified: https://github.com/MetaGLM/glm-realtime-sdk and
        // https://docs.bigmodel.cn/cn/asyncapi/realtime
        assert_eq!(REALTIME_BASE, "wss://open.bigmodel.cn/api/paas/v4/realtime");
    }

    #[test]
    fn realtime_url_is_buildable_from_endpoint_config() {
        let url = EndpointConfig::default().url(&ApiBase::Realtime, "");
        assert_eq!(url, REALTIME_BASE);
        // join_url must not mangle the wss:// scheme.
        assert!(url.starts_with("wss://"));
    }

    #[test]
    fn build_query_appends_percent_encoded_pairs() {
        let url = build_query(
            "https://open.bigmodel.cn/api/paas/v4/files",
            [("purpose", "batch".to_string()), ("limit", "2".to_string())],
        );
        assert_eq!(
            url,
            "https://open.bigmodel.cn/api/paas/v4/files?purpose=batch&limit=2"
        );
    }

    #[test]
    fn build_query_without_params_returns_base() {
        let url = build_query(
            "https://open.bigmodel.cn/api/paas/v4/files",
            Vec::<(String, String)>::new(),
        );
        assert_eq!(url, "https://open.bigmodel.cn/api/paas/v4/files");
    }

    #[test]
    fn build_query_falls_back_for_malformed_base() {
        // A non-parseable base must not panic.
        let url = build_query("not a url", [("limit", "5".to_string())]);
        assert_eq!(url, "not a url?limit=5");
    }
}
