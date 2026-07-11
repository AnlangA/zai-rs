//! Service facades (plan P02.11).
//!
//! Each facade is a zero-sized handle borrowed from [`Services`], which itself
//! borrows a [`ZaiClient`]. Obtaining a facade (e.g. `client.services().chat()`)
//! is free; it carries no state of its own and dispatches through the shared
//! `ClientInner`. The facades are the public surface callers use:
//!
//! ```text
//! let client = ZaiClient::builder(api_key).build()?;
//! let response = client.services().chat().complete(request).await?;
//! ```
//!
//! During the P02–P05 migration window the facades are scaffolding: the typed
//! `complete`/`generate`/… methods land in P04–P06. P02 establishes the facade
//! structure so every family has a single, owned-by-`ZaiClient` entry point.

use crate::client::ZaiClient;

/// The collection of service facades. Obtain via [`ZaiClient::services`].
///
/// This is a zero-sized handle — it only re-borrows the client, so cloning or
/// dropping it is free.
#[derive(Clone)]
pub struct Services {
    pub(super) client: ZaiClient,
}

impl Services {
    pub(super) fn new(client: ZaiClient) -> Self {
        Self { client }
    }

    /// The owning client (rarely needed directly; facades delegate to it).
    pub fn client(&self) -> &ZaiClient {
        &self.client
    }

    // --- facades (P02 establishes the surface; bodies arrive in P04–P06) ---

    pub fn chat(&self) -> ChatService {
        ChatService { svc: self.clone() }
    }
    pub fn images(&self) -> ImagesService {
        ImagesService { svc: self.clone() }
    }
    pub fn videos(&self) -> VideosService {
        VideosService { svc: self.clone() }
    }
    pub fn audio(&self) -> AudioService {
        AudioService { svc: self.clone() }
    }
    pub fn embeddings(&self) -> EmbeddingsService {
        EmbeddingsService { svc: self.clone() }
    }
    pub fn rerank(&self) -> RerankService {
        RerankService { svc: self.clone() }
    }
    pub fn tokenizer(&self) -> TokenizerService {
        TokenizerService { svc: self.clone() }
    }
    pub fn moderation(&self) -> ModerationService {
        ModerationService { svc: self.clone() }
    }
    pub fn files(&self) -> FilesService {
        FilesService { svc: self.clone() }
    }
    pub fn batches(&self) -> BatchesService {
        BatchesService { svc: self.clone() }
    }
    pub fn knowledge(&self) -> KnowledgeService {
        KnowledgeService { svc: self.clone() }
    }
    pub fn agents(&self) -> AgentsService {
        AgentsService { svc: self.clone() }
    }
    pub fn tools(&self) -> ToolsService {
        ToolsService { svc: self.clone() }
    }
    pub fn assistants(&self) -> AssistantsService {
        AssistantsService { svc: self.clone() }
    }
    pub fn applications(&self) -> ApplicationsService {
        ApplicationsService { svc: self.clone() }
    }
    pub fn tasks(&self) -> TasksService {
        TasksService { svc: self.clone() }
    }
    pub fn zrag(&self) -> ZragService {
        ZragService { svc: self.clone() }
    }
    pub fn usage(&self) -> UsageService {
        UsageService { svc: self.clone() }
    }
}

macro_rules! facade {
    ($name:ident, $doc:expr) => {
        #[doc = $doc]
        #[derive(Clone)]
        pub struct $name {
            pub(super) svc: Services,
        }

        impl $name {
            /// Borrow the owning client.
            pub fn client(&self) -> &ZaiClient {
                self.svc.client()
            }
        }
    };
}

facade!(ChatService, "Chat completions service facade.");
facade!(ImagesService, "Image generation service facade.");
facade!(VideosService, "Video generation service facade.");
facade!(AudioService, "Audio (ASR/TTS/voice) service facade.");
facade!(EmbeddingsService, "Embeddings service facade.");
facade!(RerankService, "Rerank service facade.");
facade!(TokenizerService, "Tokenizer service facade.");
facade!(ModerationService, "Moderation service facade.");
facade!(FilesService, "Files service facade.");
facade!(BatchesService, "Batches service facade.");
facade!(KnowledgeService, "Knowledge-base service facade.");
facade!(AgentsService, "Agent v1 service facade.");
facade!(
    ToolsService,
    "Tools (web search / layout / reader) service facade."
);
facade!(AssistantsService, "Assistant service facade.");
facade!(ApplicationsService, "LLM-application service facade.");
facade!(TasksService, "Async-task service facade.");
facade!(ZragService, "Zrag service facade.");
facade!(UsageService, "Coding-plan usage service facade.");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn facades_are_zero_cost_and_share_inner() {
        let client = ZaiClient::builder("abcdefghij.0123456789abcdef")
            .build()
            .unwrap();
        // Creating 1000 facades must not allocate extra reqwest clients or copy
        // the secret: all facades share the one Arc<ClientInner>.
        let mut handles = Vec::new();
        for _ in 0..1000 {
            handles.push(client.services().chat());
        }
        // Each facade re-borrows the same client; dropping them is free.
        assert_eq!(handles.len(), 1000);
        // The underlying secret is still reachable and redacted.
        assert_eq!(format!("{:?}", client.secret()), "[REDACTED]");
    }
}
