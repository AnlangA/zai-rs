//! # AI Model Type Definitions
//!
//! Defines all available AI model types for the Zhipu AI API, together with
//! their capability markers and message-type bindings.
//!
//! # Naming convention
//!
//! Each model struct mirrors the vendor's model string as closely as Rust's
//! identifier rules allow: the `GLM` prefix and version segments use
//! underscores for the version dot (e.g. `glm-4.5-air` → `GLM4_5_air`), and
//! the tier suffix is appended verbatim (e.g. `glm-4.7-flashx` →
//! `GLM4_7_flashx`). Because these intentionally echo API
//! strings rather than follow Rust's `UpperCamelCase` convention, every model
//! struct carries `#[allow(non_camel_case_types)]`.
//!
//! # Model Categories
//!
//! ## Text Models
//!
//! | Model | Struct | Thinking | ReasoningEffort | Async | ToolStream |
//! |-------|--------|----------|-----------------|-------|------------|
//! | glm-5.3 | [`GLM5_3`] | yes | yes | yes | yes |
//! | glm-5.2 | [`GLM5_2`] | yes | yes | yes | yes |
//! | glm-5.1 | [`GLM5_1`] | yes | no | yes | yes |
//! | glm-5.1-highspeed | [`GLM5_1_highspeed`] | yes | no | yes | yes |
//! | glm-5 | [`GLM5`] | yes | no | yes | yes |
//! | glm-5-turbo | [`GLM5_turbo`] | yes | no | yes | yes |
//! | glm-4.7 | [`GLM4_7`] | yes | no | yes | yes |
//! | glm-4.7-flash | [`GLM4_7_flash`] | yes | no | no | no |
//! | glm-4.7-flashx | [`GLM4_7_flashx`] | yes | no | no | no |
//! | glm-4.6 | [`GLM4_6`] | yes | no | yes | yes |
//! | glm-4.5-flash | [`GLM4_5_flash`] | yes | no | yes | no |
//! | glm-4.5-air | [`GLM4_5_air`] | yes | no | yes | no |
//! | glm-4.5-airx | [`GLM4_5_airx`] | yes | no | yes | no |
//! | glm-4-flash-250414 | [`GLM4_flash_250414`] | no | no | yes | no |
//! | glm-4-flashx-250414 | [`GLM4_flashx_250414`] | no | no | yes | no |
//!
//! Text models accept the complete chat tool union and `response_format`.
//!
//! GLM-5.3 always thinks: `thinking.type = "disabled"` is rejected by request
//! validation (`thinking_cannot_be_disabled`), and its `reasoning_effort` is
//! frozen to `low` / `high` / `max`. All other thinking-capable models accept
//! both thinking modes, and GLM-5.2 accepts every
//! [`ReasoningEffort`](crate::model::tools::ReasoningEffort) level.
//!
//! ## Vision Models
//!
//! | Model | Struct | Thinking | Message Type |
//! |-------|--------|----------|--------------|
//! | autoglm-phone | [`autoglm_phone`] | no | [`VisionMessage`] |
//! | glm-4.6v | [`GLM4_6v`] | no | [`VisionMessage`] |
//! | glm-4.6v-flash | [`GLM4_6v_flash`] | no | [`VisionMessage`] |
//! | glm-4.6v-flashx | [`GLM4_6v_flashx`] | no | [`VisionMessage`] |
//! | glm-4v-flash | [`GLM4v_flash`] | no | [`VisionMessage`] |
//! | glm-4.1v-thinking-flash | [`GLM4_1v_thinking_flash`] | yes | [`VisionMessage`] |
//! | glm-4.1v-thinking-flashx | [`GLM4_1v_thinking_flashx`] | yes | [`VisionMessage`] |
//! | glm-5v-turbo | [`GLM5V_turbo`] | yes | [`VisionMessage`] |
//!
//! Vision models accept function tools only and do not expose
//! `response_format`.
//!
//! ## Voice Models
//!
//! | Model | Struct | Message Type |
//! |-------|--------|--------------|
//! | glm-4-voice | [`GLM4_voice`] | [`VoiceMessage`] |
//!
//! `GLM4_voice` exposes `watermark_enabled`, but not tools, `tool_choice`, or
//! `response_format`.
//!
//! ## Realtime Models
//!
//! | Model | Struct |
//! |-------|--------|
//! | glm-realtime-flash | [`GLM_realtime_flash`] |
//! | glm-realtime-air | [`GLM_realtime_air`] |
//!
//! # Usage
//!
//! ```
//! use zai_rs::model::{
//!     chat::ChatCompletion,
//!     chat_message_types::TextMessage,
//!     chat_models::GLM5_2,
//! };
//!
//! let model = GLM5_2 {};
//! let messages = TextMessage::user("Hello");
//! let request = ChatCompletion::new(model, messages);
//! ```
//!
//! # Frozen capabilities
//!
//! Synchronous, asynchronous, and request-schema capabilities are sealed to
//! the model ids listed above. Downstream crates cannot opt an arbitrary
//! identifier into these operations.
//!
//! # Registry maintenance
//!
//! Chat, vision, and voice models are declared once in a private registry
//! below. Each entry emits the public zero-sized type, wire id, message binding,
//! sealed/public capability traits, and request-schema family. A checked-in
//! snapshot keeps that declaration reviewable without exposing a runtime
//! registry as public API.

use super::{
    chat_message_types::{TextMessage, VisionMessage, VoiceMessage},
    tools::{Function, Tools},
    traits::{define_model_type, impl_message_binding, *},
};

macro_rules! impl_registered_capability {
    ($model:ident, Chat) => {
        impl super::traits::sealed::Chat for $model {}
        impl Chat for $model {}
    };
    ($model:ident, AsyncChat) => {
        impl super::traits::sealed::AsyncChat for $model {}
        impl AsyncChat for $model {}
    };
    ($model:ident, $capability:ident) => {
        impl $capability for $model {}
    };
}

macro_rules! impl_registered_capabilities {
    ($model:ident: $($capability:ident),+ $(,)?) => {
        $(impl_registered_capability!($model, $capability);)+
    };
}

macro_rules! impl_registered_request_schema {
    // Text schema with a per-model thinking / reasoning-effort contract.
    // `thinking` is `toggleable` or `always_on`; `efforts` lists the accepted
    // `ReasoningEffort` levels (at least one).
    ($model:ident, text, $max_tokens:expr,
        constraints: { thinking: $thinking:ident, efforts: [$($effort:ident),+ $(,)?] }) => {
        impl super::traits::sealed::ChatRequestModel for $model {}
        impl ChatRequestModel for $model {
            const MAX_TOKENS: u32 = $max_tokens;
            const THINKING_DISABLE_SUPPORTED: bool =
                impl_registered_request_schema!(@thinking_disable $thinking);
            const REASONING_EFFORTS: &'static [super::tools::ReasoningEffort] =
                &[$(super::tools::ReasoningEffort::$effort),+];
        }
        impl ChatToolSupport for $model {
            type Tool = Tools;
        }
        impl ResponseFormatEnable for $model {}
    };
    ($model:ident, text, $max_tokens:expr) => {
        impl super::traits::sealed::ChatRequestModel for $model {}
        impl ChatRequestModel for $model {
            const MAX_TOKENS: u32 = $max_tokens;
            const THINKING_DISABLE_SUPPORTED: bool = true;
            const REASONING_EFFORTS: &'static [super::tools::ReasoningEffort] = &[];
        }
        impl ChatToolSupport for $model {
            type Tool = Tools;
        }
        impl ResponseFormatEnable for $model {}
    };
    ($model:ident, vision, $max_tokens:expr) => {
        impl super::traits::sealed::ChatRequestModel for $model {}
        impl ChatRequestModel for $model {
            const MAX_TOKENS: u32 = $max_tokens;
            const THINKING_DISABLE_SUPPORTED: bool = true;
            const REASONING_EFFORTS: &'static [super::tools::ReasoningEffort] = &[];
        }
        impl ChatToolSupport for $model {
            type Tool = Function;
        }
    };
    ($model:ident, voice, $max_tokens:expr) => {
        impl super::traits::sealed::ChatRequestModel for $model {}
        impl ChatRequestModel for $model {
            const MAX_TOKENS: u32 = $max_tokens;
            const THINKING_DISABLE_SUPPORTED: bool = true;
            const REASONING_EFFORTS: &'static [super::tools::ReasoningEffort] = &[];
        }
        impl WatermarkEnable for $model {}
    };
    (@thinking_disable toggleable) => { true };
    (@thinking_disable always_on) => { false };
}

#[cfg(test)]
struct ChatModelSnapshot {
    type_name: &'static str,
    model_id: &'static str,
    message: &'static str,
    request_schema: &'static str,
    max_tokens: u32,
    capabilities: &'static [&'static str],
    thinking: &'static str,
    reasoning_efforts: &'static [&'static str],
}

macro_rules! chat_model_registry {
    (
        $(
            $(#[$meta:meta])*
            $model:ident => {
                id: $model_id:literal,
                message: $message:ty,
                request: $request_schema:ident(
                    $max_tokens:expr
                    $(, constraints: { thinking: $thinking:ident, efforts: [$($effort:ident),* $(,)?] })?
                    $(,)?
                ),
                capabilities: [$($capability:ident),+ $(,)?],
            };
        )+
    ) => {
        $(
            define_model_type!($(#[$meta])* $model, $model_id);
            impl_message_binding!($model, $message);
            impl_registered_capabilities!($model: $($capability),+);
            impl_registered_request_schema!(
                $model, $request_schema, $max_tokens
                $(, constraints: { thinking: $thinking, efforts: [$($effort),*] })?
            );
        )+

        #[cfg(test)]
        const CHAT_MODEL_REGISTRY_SNAPSHOT: &[ChatModelSnapshot] = &[
            $(
                ChatModelSnapshot {
                    type_name: stringify!($model),
                    model_id: $model_id,
                    message: stringify!($message),
                    request_schema: stringify!($request_schema),
                    max_tokens: $max_tokens,
                    capabilities: &[$(stringify!($capability)),+],
                    thinking: chat_model_registry!(@thinking_label $($thinking)?),
                    reasoning_efforts: &[$($(stringify!($effort)),*)?],
                },
            )+
        ];
    };
    (@thinking_label) => { "toggleable" };
    (@thinking_label $kind:ident) => { stringify!($kind) };
}

chat_model_registry! {
    /// GLM-5.3 keeps thinking always on: `thinking.type` accepts only
    /// `enabled`, and `reasoning_effort` accepts `low` / `high` / `max`
    /// (default `max`). Request validation rejects
    /// [`ThinkingType::disabled()`](super::tools::ThinkingType::disabled) and
    /// effort levels outside that set before the request is sent.
    GLM5_3 => {
        id: "glm-5.3",
        message: TextMessage,
        request: text(131_072, constraints: { thinking: always_on, efforts: [Low, High, Max] }),
        capabilities: [Chat, AsyncChat, ThinkEnable, ReasoningEffortEnable, ToolStreamEnable],
    };
    GLM5_2 => {
        id: "glm-5.2",
        message: TextMessage,
        request: text(
            131_072,
            constraints: { thinking: toggleable, efforts: [Max, Xhigh, High, Medium, Low, Minimal, None] },
        ),
        capabilities: [Chat, AsyncChat, ThinkEnable, ReasoningEffortEnable, ToolStreamEnable],
    };
    GLM5_1 => {
        id: "glm-5.1",
        message: TextMessage,
        request: text(131_072),
        capabilities: [Chat, AsyncChat, ThinkEnable, ToolStreamEnable],
    };
    #[allow(non_camel_case_types)]
    GLM5_1_highspeed => {
        id: "glm-5.1-highspeed",
        message: TextMessage,
        request: text(131_072),
        capabilities: [Chat, AsyncChat, ThinkEnable, ToolStreamEnable],
    };
    #[allow(non_camel_case_types)]
    GLM5_turbo => {
        id: "glm-5-turbo",
        message: TextMessage,
        request: text(131_072),
        capabilities: [Chat, AsyncChat, ThinkEnable, ToolStreamEnable],
    };
    GLM5 => {
        id: "glm-5",
        message: TextMessage,
        request: text(131_072),
        capabilities: [Chat, AsyncChat, ThinkEnable, ToolStreamEnable],
    };
    #[allow(non_camel_case_types)]
    GLM5V_turbo => {
        id: "glm-5v-turbo",
        message: VisionMessage,
        request: vision(131_072),
        capabilities: [Chat, AsyncChat, ThinkEnable],
    };
    GLM4_7 => {
        id: "glm-4.7",
        message: TextMessage,
        request: text(131_072),
        capabilities: [Chat, AsyncChat, ThinkEnable, ToolStreamEnable],
    };
    #[allow(non_camel_case_types)]
    GLM4_7_flash => {
        id: "glm-4.7-flash",
        message: TextMessage,
        request: text(131_072),
        capabilities: [Chat, ThinkEnable],
    };
    #[allow(non_camel_case_types)]
    GLM4_7_flashx => {
        id: "glm-4.7-flashx",
        message: TextMessage,
        request: text(131_072),
        capabilities: [Chat, ThinkEnable],
    };
    GLM4_6 => {
        id: "glm-4.6",
        message: TextMessage,
        request: text(131_072),
        capabilities: [Chat, AsyncChat, ThinkEnable, ToolStreamEnable],
    };
    #[allow(non_camel_case_types)]
    GLM4_5_flash => {
        id: "glm-4.5-flash",
        message: TextMessage,
        request: text(131_072),
        capabilities: [Chat, AsyncChat, ThinkEnable],
    };
    #[allow(non_camel_case_types)]
    GLM4_5_air => {
        id: "glm-4.5-air",
        message: TextMessage,
        request: text(131_072),
        capabilities: [Chat, AsyncChat, ThinkEnable],
    };
    #[allow(non_camel_case_types)]
    GLM4_5_airx => {
        id: "glm-4.5-airx",
        message: TextMessage,
        request: text(131_072),
        capabilities: [Chat, AsyncChat, ThinkEnable],
    };
    #[allow(non_camel_case_types)]
    GLM4_flash_250414 => {
        id: "glm-4-flash-250414",
        message: TextMessage,
        request: text(131_072),
        capabilities: [Chat, AsyncChat],
    };
    #[allow(non_camel_case_types)]
    GLM4_flashx_250414 => {
        id: "glm-4-flashx-250414",
        message: TextMessage,
        request: text(131_072),
        capabilities: [Chat, AsyncChat],
    };
    #[allow(non_camel_case_types)]
    autoglm_phone => {
        id: "autoglm-phone",
        message: VisionMessage,
        request: vision(131_072),
        capabilities: [Chat],
    };
    #[allow(non_camel_case_types)]
    GLM4_6v => {
        id: "glm-4.6v",
        message: VisionMessage,
        request: vision(131_072),
        capabilities: [Chat, AsyncChat],
    };
    #[allow(non_camel_case_types)]
    GLM4_6v_flash => {
        id: "glm-4.6v-flash",
        message: VisionMessage,
        request: vision(131_072),
        capabilities: [Chat, AsyncChat],
    };
    #[allow(non_camel_case_types)]
    GLM4_6v_flashx => {
        id: "glm-4.6v-flashx",
        message: VisionMessage,
        request: vision(131_072),
        capabilities: [Chat, AsyncChat],
    };
    #[allow(non_camel_case_types)]
    GLM4v_flash => {
        id: "glm-4v-flash",
        message: VisionMessage,
        request: vision(131_072),
        capabilities: [Chat, AsyncChat],
    };
    #[allow(non_camel_case_types)]
    GLM4_1v_thinking_flash => {
        id: "glm-4.1v-thinking-flash",
        message: VisionMessage,
        request: vision(131_072),
        capabilities: [Chat, AsyncChat, ThinkEnable],
    };
    #[allow(non_camel_case_types)]
    GLM4_1v_thinking_flashx => {
        id: "glm-4.1v-thinking-flashx",
        message: VisionMessage,
        request: vision(131_072),
        capabilities: [Chat, AsyncChat, ThinkEnable],
    };
    #[allow(non_camel_case_types)]
    GLM4_voice => {
        id: "glm-4-voice",
        message: VoiceMessage,
        request: voice(4_096),
        capabilities: [Chat, AsyncChat],
    };
}

// The wire markers remain available without the optional `realtime` feature,
// while their WebSocket capability trait lives in the feature-gated module.
// A callback registry lets both projections consume one private type/id list
// without introducing a public runtime registry or coupling model ids to
// transport configuration.
macro_rules! realtime_model_registry {
    ($consumer:ident) => {
        $consumer! {
            #[allow(non_camel_case_types)]
            GLM_realtime_flash => "glm-realtime-flash";
            #[allow(non_camel_case_types)]
            GLM_realtime_air => "glm-realtime-air";
        }
    };
}
#[cfg(feature = "realtime")]
pub(crate) use realtime_model_registry;

macro_rules! define_realtime_wire_models {
    (
        $(
            $(#[$meta:meta])*
            $model:ident => $model_id:literal;
        )+
    ) => {
        $(define_model_type!($(#[$meta])* $model, $model_id);)+
    };
}

realtime_model_registry!(define_realtime_wire_models);

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt::Write as _;

    fn assert_sync_model<N, M>()
    where
        N: Chat,
        (N, M): Bounded,
    {
    }

    fn assert_async_model<N, M>()
    where
        N: AsyncChat,
        (N, M): Bounded,
    {
    }

    fn assert_text_request_schema<N>()
    where
        N: ChatRequestModel + ChatToolSupport<Tool = Tools> + ResponseFormatEnable,
    {
    }

    fn assert_vision_request_schema<N>()
    where
        N: ChatRequestModel + ChatToolSupport<Tool = Function>,
    {
    }

    fn assert_audio_request_schema<N>()
    where
        N: ChatRequestModel + WatermarkEnable,
    {
    }

    #[test]
    fn private_registry_matches_reviewed_snapshot() {
        let mut actual = String::new();
        for model in CHAT_MODEL_REGISTRY_SNAPSHOT {
            writeln!(
                actual,
                "{}|{}|{}|{}|{}|{}|{}|{}",
                model.type_name,
                model.model_id,
                model.message,
                model.request_schema,
                model.max_tokens,
                model.capabilities.join(","),
                model.thinking,
                model.reasoning_efforts.join(",")
            )
            .expect("writing to a String cannot fail");
        }

        assert_eq!(actual, include_str!("snapshots/chat_models.txt"));
    }

    #[test]
    fn request_schema_capabilities_are_typed_by_model_family() {
        assert_text_request_schema::<GLM5_3>();
        assert_text_request_schema::<GLM5_2>();
        assert_text_request_schema::<GLM5_1>();
        assert_text_request_schema::<GLM5_1_highspeed>();
        assert_text_request_schema::<GLM5_turbo>();
        assert_text_request_schema::<GLM5>();
        assert_text_request_schema::<GLM4_7>();
        assert_text_request_schema::<GLM4_7_flash>();
        assert_text_request_schema::<GLM4_7_flashx>();
        assert_text_request_schema::<GLM4_6>();
        assert_text_request_schema::<GLM4_5_flash>();
        assert_text_request_schema::<GLM4_5_air>();
        assert_text_request_schema::<GLM4_5_airx>();
        assert_text_request_schema::<GLM4_flash_250414>();
        assert_text_request_schema::<GLM4_flashx_250414>();

        assert_vision_request_schema::<GLM5V_turbo>();
        assert_vision_request_schema::<autoglm_phone>();
        assert_vision_request_schema::<GLM4_6v>();
        assert_vision_request_schema::<GLM4_6v_flash>();
        assert_vision_request_schema::<GLM4_6v_flashx>();
        assert_vision_request_schema::<GLM4v_flash>();
        assert_vision_request_schema::<GLM4_1v_thinking_flash>();
        assert_vision_request_schema::<GLM4_1v_thinking_flashx>();

        assert_audio_request_schema::<GLM4_voice>();
        assert_eq!(GLM5_3::MAX_TOKENS, 131_072);
        assert_eq!(GLM5_2::MAX_TOKENS, 131_072);
        assert_eq!(GLM4_voice::MAX_TOKENS, 4_096);
    }

    #[test]
    fn thinking_and_effort_constraints_match_the_frozen_contract() {
        use super::super::tools::ReasoningEffort;

        // Compared as values (not `assert!`) so the frozen contract stays a
        // reviewable expectation rather than a foldable constant assertion.
        assert_eq!(
            (
                GLM5_3::THINKING_DISABLE_SUPPORTED,
                GLM5_2::THINKING_DISABLE_SUPPORTED,
                GLM5_1::THINKING_DISABLE_SUPPORTED,
                GLM4_1v_thinking_flash::THINKING_DISABLE_SUPPORTED,
            ),
            (false, true, true, true)
        );

        // GLM-5.3 always thinks and only accepts low / high / max.
        assert_eq!(
            GLM5_3::REASONING_EFFORTS,
            [
                ReasoningEffort::Low,
                ReasoningEffort::High,
                ReasoningEffort::Max
            ]
        );

        // GLM-5.2 keeps both thinking modes and every effort level.
        assert_eq!(
            GLM5_2::REASONING_EFFORTS,
            [
                ReasoningEffort::Max,
                ReasoningEffort::Xhigh,
                ReasoningEffort::High,
                ReasoningEffort::Medium,
                ReasoningEffort::Low,
                ReasoningEffort::Minimal,
                ReasoningEffort::None,
            ]
        );

        // Models without ReasoningEffortEnable declare no levels.
        assert_eq!(GLM5_1::REASONING_EFFORTS, []);
    }

    #[test]
    fn official_chat_model_names_match_snapshot() {
        let models = [
            String::from(GLM5_3 {}),
            String::from(GLM5_2 {}),
            String::from(GLM5_1 {}),
            String::from(GLM5_1_highspeed {}),
            String::from(GLM5_turbo {}),
            String::from(GLM5 {}),
            String::from(GLM4_7 {}),
            String::from(GLM4_7_flash {}),
            String::from(GLM4_7_flashx {}),
            String::from(GLM4_6 {}),
            String::from(GLM4_5_air {}),
            String::from(GLM4_5_airx {}),
            String::from(GLM4_5_flash {}),
            String::from(GLM4_flash_250414 {}),
            String::from(GLM4_flashx_250414 {}),
        ];

        assert_eq!(
            models,
            [
                "glm-5.3",
                "glm-5.2",
                "glm-5.1",
                "glm-5.1-highspeed",
                "glm-5-turbo",
                "glm-5",
                "glm-4.7",
                "glm-4.7-flash",
                "glm-4.7-flashx",
                "glm-4.6",
                "glm-4.5-air",
                "glm-4.5-airx",
                "glm-4.5-flash",
                "glm-4-flash-250414",
                "glm-4-flashx-250414",
            ]
        );
    }

    #[test]
    fn official_vision_model_names_match_snapshot() {
        let models = [
            String::from(GLM5V_turbo {}),
            String::from(GLM4_6v {}),
            String::from(autoglm_phone {}),
            String::from(GLM4_6v_flash {}),
            String::from(GLM4_6v_flashx {}),
            String::from(GLM4v_flash {}),
            String::from(GLM4_1v_thinking_flashx {}),
            String::from(GLM4_1v_thinking_flash {}),
        ];
        assert_eq!(
            models,
            [
                "glm-5v-turbo",
                "glm-4.6v",
                "autoglm-phone",
                "glm-4.6v-flash",
                "glm-4.6v-flashx",
                "glm-4v-flash",
                "glm-4.1v-thinking-flashx",
                "glm-4.1v-thinking-flash",
            ]
        );
    }

    #[test]
    fn capability_markers_cover_the_frozen_sync_and_async_enums() {
        assert_sync_model::<GLM5_3, TextMessage>();
        assert_sync_model::<GLM5_2, TextMessage>();
        assert_sync_model::<GLM5_1, TextMessage>();
        assert_sync_model::<GLM5_1_highspeed, TextMessage>();
        assert_sync_model::<GLM5_turbo, TextMessage>();
        assert_sync_model::<GLM5, TextMessage>();
        assert_sync_model::<GLM4_7, TextMessage>();
        assert_sync_model::<GLM4_7_flash, TextMessage>();
        assert_sync_model::<GLM4_7_flashx, TextMessage>();
        assert_sync_model::<GLM4_6, TextMessage>();
        assert_sync_model::<GLM4_5_air, TextMessage>();
        assert_sync_model::<GLM4_5_airx, TextMessage>();
        assert_sync_model::<GLM4_5_flash, TextMessage>();
        assert_sync_model::<GLM4_flash_250414, TextMessage>();
        assert_sync_model::<GLM4_flashx_250414, TextMessage>();

        assert_async_model::<GLM5_3, TextMessage>();
        assert_async_model::<GLM5_2, TextMessage>();
        assert_async_model::<GLM5_1, TextMessage>();
        assert_async_model::<GLM5_1_highspeed, TextMessage>();
        assert_async_model::<GLM5_turbo, TextMessage>();
        assert_async_model::<GLM5, TextMessage>();
        assert_async_model::<GLM4_7, TextMessage>();
        assert_async_model::<GLM4_6, TextMessage>();
        assert_async_model::<GLM4_5_air, TextMessage>();
        assert_async_model::<GLM4_5_airx, TextMessage>();
        assert_async_model::<GLM4_5_flash, TextMessage>();
        assert_async_model::<GLM4_flash_250414, TextMessage>();
        assert_async_model::<GLM4_flashx_250414, TextMessage>();

        assert_sync_model::<GLM5V_turbo, VisionMessage>();
        assert_sync_model::<GLM4_6v, VisionMessage>();
        assert_sync_model::<autoglm_phone, VisionMessage>();
        assert_sync_model::<GLM4_6v_flash, VisionMessage>();
        assert_sync_model::<GLM4_6v_flashx, VisionMessage>();
        assert_sync_model::<GLM4v_flash, VisionMessage>();
        assert_sync_model::<GLM4_1v_thinking_flash, VisionMessage>();
        assert_sync_model::<GLM4_1v_thinking_flashx, VisionMessage>();

        assert_async_model::<GLM5V_turbo, VisionMessage>();
        assert_async_model::<GLM4_6v, VisionMessage>();
        assert_async_model::<GLM4_6v_flash, VisionMessage>();
        assert_async_model::<GLM4_6v_flashx, VisionMessage>();
        assert_async_model::<GLM4v_flash, VisionMessage>();
        assert_async_model::<GLM4_1v_thinking_flash, VisionMessage>();
        assert_async_model::<GLM4_1v_thinking_flashx, VisionMessage>();

        assert_sync_model::<GLM4_voice, VoiceMessage>();
        assert_async_model::<GLM4_voice, VoiceMessage>();
    }
}
