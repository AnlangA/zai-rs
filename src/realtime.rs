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
//!
//! ## Transport and backpressure policy
//!
//! [`crate::realtime::RealtimeTransportConfig`] provides checked timeouts, independent queue
//! and broadcast capacities, and the built-in WebSocket frame limit. A client
//! snapshots its default into each new [`crate::realtime::SessionBuilder`];
//! [`crate::realtime::SessionBuilder::with_transport_config`] can override that
//! snapshot for one session.
//!
//! ```rust
//! use std::time::Duration;
//! use zai_rs::realtime::{RealtimeClient, RealtimeTransportConfig};
//!
//! let policy = RealtimeTransportConfig::builder()
//!     .outbound_queue_timeout(Duration::from_secs(5))
//!     .event_buffer_capacity(4)
//!     .try_build()?;
//! let client = RealtimeClient::new("id.0123456789abcdef")
//!     .with_transport_config(policy);
//! # Ok::<(), zai_rs::ZaiError>(())
//! ```
//!
//! Built-in Tungstenite sessions apply every setting. An already-connected
//! transport supplied through
//! [`crate::realtime::SessionBuilder::build_with_transport`] applies only
//! SDK-owned session policy (outbound admission, event/audio buffers, inbound
//! idle, and outer send/close deadlines); the SDK cannot impose connect, Pong,
//! writer, or frame settings on third-party transport code. The 8 MiB message
//! and queue-byte ceilings and 4 MiB media ceiling remain hard safety limits.

pub mod audio;
pub mod client;
mod config;
pub mod events;
pub mod jwt;
pub mod protocol;
pub mod session;
pub mod transport;

pub use audio::{InputAudioFormat, OutputAudioFormat};
pub use client::{AuthMode, RealtimeClient};
pub use config::{RealtimeTransportConfig, RealtimeTransportConfigBuilder};
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

use crate::model::{chat_models::realtime_model_registry, traits::ModelName};

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
///
/// The capability is also sealed against downstream implementations:
///
/// ```compile_fail
/// use zai_rs::{model::traits::ModelName, realtime::RealtimeModel};
///
/// #[derive(serde::Serialize)]
/// struct UnofficialRealtime;
///
/// impl From<UnofficialRealtime> for String {
///     fn from(_: UnofficialRealtime) -> Self {
///         "unofficial-realtime".to_owned()
///     }
/// }
///
/// impl ModelName for UnofficialRealtime {
///     const NAME: &'static str = "unofficial-realtime";
/// }
///
/// impl RealtimeModel for UnofficialRealtime {}
/// ```
pub trait RealtimeModel: ModelName + sealed::Sealed {}

mod sealed {
    pub trait Sealed {}
}

macro_rules! impl_realtime_model_registry {
    (
        $(
            $(#[$meta:meta])*
            $model:ident => $model_id:literal;
        )+
    ) => {
        $(
            impl sealed::Sealed for crate::model::$model {}
            impl RealtimeModel for crate::model::$model {}
        )+

        #[cfg(test)]
        const REALTIME_MODEL_REGISTRY_SNAPSHOT: &[(&str, &str, &str)] = &[
            $(
                (
                    stringify!($model),
                    $model_id,
                    stringify!(RealtimeModel),
                ),
            )+
        ];

        #[cfg(test)]
        mod model_registry_tests {
            use super::*;
            use std::fmt::Write as _;

            fn assert_registration<M>(expected_id: &str)
            where
                M: RealtimeModel + Default,
            {
                assert_eq!(M::NAME, expected_id);
                assert_eq!(Into::<String>::into(M::default()), expected_id);
                assert_eq!(
                    serde_json::to_value(M::default()).expect("model id must serialize"),
                    serde_json::Value::String(expected_id.to_owned()),
                );
            }

            #[test]
            fn generated_wire_and_realtime_capability_contracts_hold() {
                $(assert_registration::<crate::model::$model>($model_id);)+
            }

            #[test]
            fn private_registry_matches_reviewed_snapshot() {
                let mut actual = String::new();
                for (type_name, model_id, capability) in REALTIME_MODEL_REGISTRY_SNAPSHOT {
                    writeln!(actual, "{type_name}|{model_id}|{capability}")
                        .expect("writing to a String cannot fail");
                }

                assert_eq!(
                    actual,
                    include_str!("model/snapshots/realtime_models.txt")
                );
            }
        }
    };
}

realtime_model_registry!(impl_realtime_model_registry);
