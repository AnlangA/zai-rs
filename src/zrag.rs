//! ZRAG multimodal knowledge retrieval and streaming agent chat.
//!
//! ZRAG uses its own [`crate::client::ApiFamily::Zrag`] endpoint family. The
//! request-centric API follows the rest of the crate: construct a typed
//! [`crate::zrag::ZragRetrieveRequest`] or [`crate::zrag::ZragChatRequest`],
//! validate it locally, then dispatch it through a [`crate::client::ZaiClient`].
//!
//! ```rust,no_run
//! use zai_rs::{
//!     ZaiClient,
//!     zrag::{ZragKnowledge, ZragRetrieveRequest},
//! };
//!
//! # async fn retrieve(client: &ZaiClient) -> zai_rs::ZaiResult<()> {
//! let response = ZragRetrieveRequest::new(vec![ZragKnowledge::new("knowledge-id")])
//!     .with_query("How do I authenticate?")
//!     .send_via(client)
//!     .await?;
//!
//! for content in response
//!     .data()
//!     .and_then(|data| data.contents())
//!     .unwrap_or_default()
//! {
//!     if let Some(text) = content.text() {
//!         println!("{text}");
//!     }
//! }
//! # Ok(())
//! # }
//! ```
//!
//! Agent chat is stream-only. Its JSON `type=done` event is yielded to the
//! caller before the stream terminates:
//!
//! ```rust,no_run
//! use zai_rs::{
//!     ZaiClient,
//!     zrag::{AgentStreamEvent, ZragChatMessage, ZragChatRequest, ZragChatRetrieval},
//! };
//!
//! # async fn chat(client: &ZaiClient) -> zai_rs::ZaiResult<()> {
//! let request = ZragChatRequest::new(
//!     vec![ZragChatMessage::user("What is our leave policy?")],
//!     ZragChatRetrieval::new(vec!["knowledge-id".to_owned()]),
//! )
//! .with_session_id("optional-continuation-session");
//! let mut events = request.stream_via(client).await?;
//! while let Some(event) = events.next().await.transpose()? {
//!     if let AgentStreamEvent::Answer(answer) = event {
//!         if let Some(text) = answer.data() {
//!             print!("{text}");
//!         }
//!     }
//! }
//! # Ok(())
//! # }
//! ```

mod chat;
mod retrieve;

pub use chat::{
    AgentCompletionTokenDetails, AgentDoneEvent, AgentPromptTokenDetails, AgentStreamEvent,
    AgentTextEvent, AgentToolCallData, AgentToolCallEvent, AgentToolResultData,
    AgentToolResultEvent, AgentToolResultStatus, AgentUnknownEvent, AgentUsage, ZragChatContent,
    ZragChatContentPart, ZragChatEvent, ZragChatImageUrl, ZragChatMessage, ZragChatMessageRole,
    ZragChatRequest, ZragChatRetrieval, ZragChatStream, ZragEventStream,
};

pub use retrieve::{
    ZragFilterValueType, ZragImagePart, ZragIndexTypeFilter, ZragKnowledge, ZragMedia,
    ZragQaIntervention, ZragRecallMethod, ZragRetrieveContent, ZragRetrieveData,
    ZragRetrieveMessage, ZragRetrieveMessageRole, ZragRetrieveMetadata, ZragRetrieveRequest,
    ZragRetrieveResponse, ZragRewrittenQuery, ZragSearchFilters, ZragTagFilter,
    ZragTagFilterOperator, ZragUrl,
};
