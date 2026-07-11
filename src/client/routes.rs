//! Canonical HTTP route registry.
//!
//! Request implementations reference these operation-level constants instead
//! of repeating an API family, method, and stringly-typed path at every call
//! site. Dynamic values are represented by [`Segment::Parameter`] and are
//! still encoded by [`EndpointConfig`](super::EndpointConfig).

use super::ApiFamily;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Segment {
    Static(&'static str),
    Parameter,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Route {
    operation_id: &'static str,
    method: &'static str,
    family: ApiFamily,
    segments: &'static [Segment],
}

impl Route {
    const fn new(
        operation_id: &'static str,
        method: &'static str,
        family: ApiFamily,
        segments: &'static [Segment],
    ) -> Self {
        Self {
            operation_id,
            method,
            family,
            segments,
        }
    }

    pub(crate) const fn operation_id(self) -> &'static str {
        self.operation_id
    }

    pub(crate) const fn method(self) -> &'static str {
        self.method
    }

    pub(crate) const fn family(self) -> ApiFamily {
        self.family
    }

    pub(crate) const fn segments(self) -> &'static [Segment] {
        self.segments
    }
}

macro_rules! route {
    ($(#[$meta:meta])* $name:ident, $operation_id:literal, $method:literal, $family:ident, [$($segment:expr),* $(,)?]) => {
        $(#[$meta])*
        pub(crate) const $name: Route = Route::new(
            $operation_id,
            $method,
            ApiFamily::$family,
            &[$($segment),*],
        );
    };
}

use Segment::{Parameter as P, Static as S};

route!(
    #[allow(dead_code)]
    AGENTS_INVOKE,
    "agents.invoke",
    "POST",
    AgentV1,
    [S("agents")]
);
route!(
    #[allow(dead_code)]
    AGENTS_ASYNC_RESULT,
    "agents.async_result",
    "POST",
    AgentV1,
    [S("agents"), S("async-result")]
);
route!(
    #[allow(dead_code)]
    AGENTS_CONVERSATION,
    "agents.conversation",
    "POST",
    AgentV1,
    [S("agents"), S("conversation")]
);

route!(
    BATCHES_CREATE,
    "batches.create",
    "POST",
    PaasV4,
    [S("batches")]
);
route!(BATCHES_LIST, "batches.list", "GET", PaasV4, [S("batches")]);
route!(BATCHES_GET, "batches.get", "GET", PaasV4, [S("batches"), P]);
route!(
    BATCHES_CANCEL,
    "batches.cancel",
    "POST",
    PaasV4,
    [S("batches"), P, S("cancel")]
);

route!(FILES_LIST, "files.list", "GET", PaasV4, [S("files")]);
route!(FILES_UPLOAD, "files.upload", "POST", PaasV4, [S("files")]);
route!(
    FILES_GET_CONTENT,
    "files.content",
    "GET",
    PaasV4,
    [S("files"), P, S("content")]
);
route!(
    FILES_DELETE,
    "files.delete",
    "DELETE",
    PaasV4,
    [S("files"), P]
);
route!(
    FILES_OCR,
    "files.ocr",
    "POST",
    PaasV4,
    [S("files"), S("ocr")]
);
route!(
    FILES_PARSE,
    "files.parse",
    "POST",
    PaasV4,
    [S("files"), S("parser"), S("create")]
);
route!(
    FILES_PARSE_RESULT,
    "files.parse_result",
    "GET",
    PaasV4,
    [S("files"), S("parser"), S("result"), P, P]
);
route!(
    FILES_PARSE_SYNC,
    "files.parse_sync",
    "POST",
    PaasV4,
    [S("files"), S("parser"), S("sync")]
);

route!(
    KNOWLEDGE_CREATE,
    "knowledge.create",
    "POST",
    LlmApplication,
    [S("knowledge")]
);
route!(
    KNOWLEDGE_LIST,
    "knowledge.list",
    "GET",
    LlmApplication,
    [S("knowledge")]
);
route!(
    KNOWLEDGE_GET,
    "knowledge.get",
    "GET",
    LlmApplication,
    [S("knowledge"), P]
);
route!(
    KNOWLEDGE_UPDATE,
    "knowledge.update",
    "PUT",
    LlmApplication,
    [S("knowledge"), P]
);
route!(
    KNOWLEDGE_DELETE,
    "knowledge.delete",
    "DELETE",
    LlmApplication,
    [S("knowledge"), P]
);
route!(
    KNOWLEDGE_CAPACITY,
    "knowledge.capacity",
    "GET",
    LlmApplication,
    [S("knowledge"), S("capacity")]
);
route!(
    KNOWLEDGE_RETRIEVE,
    "knowledge.retrieve",
    "POST",
    LlmApplication,
    [S("knowledge"), S("retrieve")]
);
route!(
    DOCUMENTS_LIST,
    "knowledge.list_documents",
    "GET",
    LlmApplication,
    [S("document")]
);
route!(
    DOCUMENTS_GET,
    "knowledge.get_document",
    "GET",
    LlmApplication,
    [S("document"), P]
);
route!(
    DOCUMENTS_DELETE,
    "knowledge.delete_document",
    "DELETE",
    LlmApplication,
    [S("document"), P]
);
route!(
    DOCUMENTS_REEMBED,
    "knowledge.reembed_document",
    "POST",
    LlmApplication,
    [S("document"), S("embedding"), P]
);
route!(
    DOCUMENTS_IMAGES,
    "knowledge.list_document_images",
    "POST",
    LlmApplication,
    [S("document"), S("slice"), S("image_list"), P]
);
route!(
    DOCUMENTS_UPLOAD,
    "knowledge.upload_document",
    "POST",
    LlmApplication,
    [S("document"), S("upload_document"), P]
);
route!(
    DOCUMENTS_UPLOAD_URL,
    "knowledge.upload_document_url",
    "POST",
    LlmApplication,
    [S("document"), S("upload_url")]
);

route!(
    CHAT_COMPLETE,
    "chat.complete",
    "POST",
    PaasV4,
    [S("chat"), S("completions")]
);
route!(
    CHAT_COMPLETE_CODING,
    "chat.complete_coding",
    "POST",
    CodingPaasV4,
    [S("chat"), S("completions")]
);
route!(
    CHAT_COMPLETE_ASYNC,
    "chat.complete_async",
    "POST",
    PaasV4,
    [S("async"), S("chat"), S("completions")]
);
route!(
    TASKS_GET,
    "tasks.get",
    "GET",
    PaasV4,
    [S("async-result"), P]
);
route!(
    IMAGES_GENERATE,
    "images.generate",
    "POST",
    PaasV4,
    [S("images"), S("generations")]
);
route!(
    IMAGES_GENERATE_ASYNC,
    "images.generate_async",
    "POST",
    PaasV4,
    [S("async"), S("images"), S("generations")]
);
route!(
    VIDEOS_GENERATE,
    "videos.generate",
    "POST",
    PaasV4,
    [S("videos"), S("generations")]
);
route!(
    AUDIO_TRANSCRIBE,
    "audio.transcribe",
    "POST",
    PaasV4,
    [S("audio"), S("transcriptions")]
);
route!(
    AUDIO_SYNTHESIZE,
    "audio.synthesize",
    "POST",
    PaasV4,
    [S("audio"), S("speech")]
);
route!(
    AUDIO_CLONE_VOICE,
    "audio.clone_voice",
    "POST",
    PaasV4,
    [S("voice"), S("clone")]
);
route!(
    AUDIO_DELETE_VOICE,
    "audio.delete_voice",
    "POST",
    PaasV4,
    [S("voice"), S("delete")]
);
route!(
    AUDIO_LIST_VOICES,
    "audio.list_voices",
    "GET",
    PaasV4,
    [S("voice"), S("list")]
);
route!(
    EMBEDDINGS_CREATE,
    "embeddings.create",
    "POST",
    PaasV4,
    [S("embeddings")]
);
route!(
    RERANK_CREATE,
    "rerank.create",
    "POST",
    PaasV4,
    [S("rerank")]
);
route!(
    TOKENIZER_COUNT,
    "tokenizer.count",
    "POST",
    PaasV4,
    [S("tokenizer")]
);
route!(
    MODERATION_CHECK,
    "moderation.check",
    "POST",
    PaasV4,
    [S("moderations")]
);

route!(
    ASSISTANTS_INVOKE,
    "assistants.invoke",
    "POST",
    PaasV4,
    [S("assistant")]
);
route!(
    ASSISTANTS_LIST,
    "assistants.list",
    "POST",
    PaasV4,
    [S("assistant"), S("list")]
);
route!(
    ASSISTANTS_CONVERSATIONS,
    "assistants.conversations",
    "POST",
    PaasV4,
    [S("assistant"), S("conversation"), S("list")]
);
route!(
    APPLICATIONS_FILE_STATS,
    "applications.file_stats",
    "POST",
    ApplicationV2,
    [S("v2"), S("application"), S("file_stat")]
);
route!(
    APPLICATIONS_UPLOAD_FILE,
    "applications.upload_file",
    "POST",
    ApplicationV2,
    [S("v2"), S("application"), S("file_upload")]
);
route!(
    APPLICATIONS_SLICE_INFO,
    "applications.slice_info",
    "POST",
    ApplicationV2,
    [S("v2"), S("application"), S("slice_info")]
);
route!(
    APPLICATIONS_CREATE_CONVERSATION,
    "applications.create_conversation",
    "POST",
    ApplicationV2,
    [S("v2"), S("application"), P, S("conversation")]
);
route!(
    APPLICATIONS_VARIABLES,
    "applications.variables",
    "GET",
    ApplicationV2,
    [S("v2"), S("application"), P, S("variables")]
);
route!(
    APPLICATIONS_HISTORY,
    "applications.history",
    "GET",
    ApplicationV2,
    [S("history_session_record"), P, P]
);
route!(
    APPLICATIONS_INVOKE,
    "applications.invoke",
    "POST",
    ApplicationV3,
    [S("v3"), S("application"), S("invoke")]
);

route!(
    TOOLS_LAYOUT,
    "tools.parse_layout",
    "POST",
    PaasV4,
    [S("layout_parsing")]
);
route!(
    TOOLS_READER,
    "tools.read_document",
    "POST",
    PaasV4,
    [S("reader")]
);
route!(
    TOOLS_WEB_SEARCH,
    "tools.web_search",
    "POST",
    PaasV4,
    [S("web_search")]
);
route!(
    #[allow(dead_code)]
    ZRAG_CHAT,
    "zrag.chat",
    "POST",
    Zrag,
    [S("agent"), S("chat")]
);
route!(
    #[allow(dead_code)]
    ZRAG_RETRIEVE,
    "zrag.retrieve",
    "POST",
    Zrag,
    [S("retrieval"), S("retrieve")]
);
route!(
    USAGE_GET,
    "usage.get",
    "GET",
    Monitor,
    [S("usage"), S("quota"), S("limit")]
);
route!(
    #[cfg_attr(not(feature = "realtime"), allow(dead_code))]
    REALTIME_CONNECT,
    "realtime.connect",
    "GET",
    Realtime,
    []
);

#[cfg(test)]
mod tests;
