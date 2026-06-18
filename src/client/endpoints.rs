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
pub const REALTIME_BASE: &str = "wss://open.bigmodel.cn/api/realtime";

/// API family selector used by [`EndpointConfig`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApiBase {
    PaasV4,
    CodingPaasV4,
    LlmApplication,
    Realtime,
    Custom(String),
}

/// Runtime-configurable API bases.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EndpointConfig {
    pub paas_v4_base: String,
    pub coding_paas_v4_base: String,
    pub llm_application_base: String,
    pub realtime_base: String,
}

impl Default for EndpointConfig {
    fn default() -> Self {
        Self {
            paas_v4_base: PAAS_V4_BASE.to_string(),
            coding_paas_v4_base: CODING_PAAS_V4_BASE.to_string(),
            llm_application_base: LLM_APPLICATION_BASE.to_string(),
            realtime_base: REALTIME_BASE.to_string(),
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

    pub fn base<'a>(&'a self, api_base: &'a ApiBase) -> &'a str {
        match api_base {
            ApiBase::PaasV4 => &self.paas_v4_base,
            ApiBase::CodingPaasV4 => &self.coding_paas_v4_base,
            ApiBase::LlmApplication => &self.llm_application_base,
            ApiBase::Realtime => &self.realtime_base,
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

pub fn default_paas_url(path: &str) -> String {
    EndpointConfig::default().url(&ApiBase::PaasV4, path)
}

pub fn default_coding_paas_url(path: &str) -> String {
    EndpointConfig::default().url(&ApiBase::CodingPaasV4, path)
}

pub fn default_llm_application_url(path: &str) -> String {
    EndpointConfig::default().url(&ApiBase::LlmApplication, path)
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
    pub const KNOWLEDGE_RETRIEVE: &str = "knowledge/retrieve";
    pub const KNOWLEDGE_CAPACITY: &str = "knowledge/capacity";
    pub const DOCUMENT: &str = "document";
    pub const DOCUMENT_UPLOAD_URL: &str = "document/upload_url";
    pub const DOCUMENT_UPLOAD_DOCUMENT: &str = "document/upload_document";
    pub const DOCUMENT_EMBEDDING: &str = "document/embedding";
    pub const DOCUMENT_SLICE_IMAGE_LIST: &str = "document/slice/image_list";
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
}
