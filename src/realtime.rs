//! # Realtime API (WebSocket)
//!
//! Realtime text and audio conversations over the GLM-Realtime WebSocket
//! protocol (`wss://open.bigmodel.cn/api/paas/v4/realtime`). The protocol types
//! also model passive-video frames, conversation history, function calls, and
//! response lifecycle events.
//!
//! Verified against the official protocol:
//! - <https://github.com/MetaGLM/glm-realtime-sdk/blob/main/GLM-Realtime-doc-for-llm.md>
//! - <https://docs.bigmodel.cn/cn/asyncapi/realtime>
//!
//! ## Quick start
//!
//! ```rust,no_run
//! use zai_rs::{
//!     model::GLM_realtime_flash,
//!     realtime::{RealtimeClient, TurnDetectionType},
//! };
//!
//! # async fn demo(key: String) -> zai_rs::ZaiResult<()> {
//! let session = RealtimeClient::new(key)
//!     .session(GLM_realtime_flash {})
//!     .turn_detection(TurnDetectionType::ServerVad)
//!     .build()
//!     .await?;
//! session.send_text("你好").await?;
//! session.create_response().await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Auth modes
//!
//! - **Bearer** (default, server-side): `Authorization: Bearer {API_KEY}`.
//! - **JWT**: `.with_jwt(ttl_seconds)` derives a short-lived token locally and
//!   sends that token, rather than the raw API key, in the WebSocket handshake.
//!   Generating the token on a trusted server before handing it to an untrusted
//!   browser or device is still the application's responsibility.

pub mod audio;
pub mod client;
pub mod events;
pub mod jwt;
pub mod protocol;
pub mod session;
pub mod transport;

pub use audio::{InputAudioFormat, OutputAudioFormat};
pub use client::{AuthMode, RealtimeClient};
pub use events::{
    ClientEvent, RealtimeConversation, RealtimeRateLimit, RealtimeSessionInfo, ServerErrorBody,
    ServerEvent,
};
pub use protocol::{
    BetaFields, ChatMode, GreetingConfig, InputAudioNoiseReduction, ItemContent, ItemType,
    NoiseReductionType, RealtimeConversationItem, RealtimeModality, RealtimeResponse, RealtimeTool,
    RealtimeUsage, RealtimeVoice, SessionConfig, TokenDetails, TurnDetection, TurnDetectionType,
};
pub use session::{RealtimeAudioChunk, RealtimeSession, SessionBuilder};
pub use transport::{RealtimeTransport, TungsteniteTransport, WsMessage};

use crate::model::traits::ModelName;

/// Marker trait for model ids usable in a realtime session.
///
/// This trait is sealed and implemented only for the current official realtime
/// model ids, so arbitrary or retired model markers cannot reach session
/// construction.
///
/// ```compile_fail
/// use zai_rs::{model::GLM4_voice, realtime::RealtimeClient};
///
/// let client = RealtimeClient::new("abc.secret-value");
/// // `GLM4_voice` is an HTTP voice-chat model, not a Realtime WebSocket model.
/// let _session = client.session(GLM4_voice {});
/// ```
pub trait RealtimeModel: ModelName + sealed::Sealed {}

impl RealtimeModel for crate::model::GLM_realtime_flash {}
impl RealtimeModel for crate::model::GLM_realtime_air {}

mod sealed {
    pub trait Sealed {}

    impl Sealed for crate::model::GLM_realtime_flash {}
    impl Sealed for crate::model::GLM_realtime_air {}
}
