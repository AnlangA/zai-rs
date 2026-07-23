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

use super::{
    chat_message_types::{TextMessage, VisionMessage, VoiceMessage},
    tools::{Function, Tools},
    traits::{define_model_type, impl_message_binding, impl_model_markers, *},
};

define_model_type!(GLM5_2, "glm-5.2");
impl_message_binding!(GLM5_2, TextMessage);
impl_model_markers!(GLM5_2: Chat, AsyncChat, ThinkEnable, ReasoningEffortEnable, ToolStreamEnable);

define_model_type!(GLM5_1, "glm-5.1");
impl_message_binding!(GLM5_1, TextMessage);
impl_model_markers!(GLM5_1: Chat, AsyncChat, ThinkEnable, ToolStreamEnable);

define_model_type!(
    #[allow(non_camel_case_types)]
    GLM5_1_highspeed,
    "glm-5.1-highspeed"
);
impl_message_binding!(GLM5_1_highspeed, TextMessage);
impl_model_markers!(GLM5_1_highspeed: Chat, AsyncChat, ThinkEnable, ToolStreamEnable);

define_model_type!(
    #[allow(non_camel_case_types)]
    GLM5_turbo,
    "glm-5-turbo"
);
impl_message_binding!(GLM5_turbo, TextMessage);
impl_model_markers!(GLM5_turbo: Chat, AsyncChat, ThinkEnable, ToolStreamEnable);

define_model_type!(GLM5, "glm-5");
impl_message_binding!(GLM5, TextMessage);
impl_model_markers!(GLM5: Chat, AsyncChat, ThinkEnable, ToolStreamEnable);

define_model_type!(
    #[allow(non_camel_case_types)]
    GLM5V_turbo,
    "glm-5v-turbo"
);
impl_message_binding!(GLM5V_turbo, VisionMessage);
impl_model_markers!(GLM5V_turbo: Chat, AsyncChat, ThinkEnable);

define_model_type!(GLM4_7, "glm-4.7");
impl_message_binding!(GLM4_7, TextMessage);
impl_model_markers!(GLM4_7: Chat, AsyncChat, ThinkEnable, ToolStreamEnable);

define_model_type!(
    #[allow(non_camel_case_types)]
    GLM4_7_flash,
    "glm-4.7-flash"
);
impl_message_binding!(GLM4_7_flash, TextMessage);
impl_model_markers!(GLM4_7_flash: Chat, ThinkEnable);

define_model_type!(
    #[allow(non_camel_case_types)]
    GLM4_7_flashx,
    "glm-4.7-flashx"
);
impl_message_binding!(GLM4_7_flashx, TextMessage);
impl_model_markers!(GLM4_7_flashx: Chat, ThinkEnable);

define_model_type!(GLM4_6, "glm-4.6");
impl_message_binding!(GLM4_6, TextMessage);
impl_model_markers!(GLM4_6: Chat, AsyncChat, ThinkEnable, ToolStreamEnable);

define_model_type!(
    #[allow(non_camel_case_types)]
    GLM4_5_flash,
    "glm-4.5-flash"
);
impl_message_binding!(GLM4_5_flash, TextMessage);
impl_model_markers!(GLM4_5_flash: Chat, AsyncChat, ThinkEnable);

define_model_type!(
    #[allow(non_camel_case_types)]
    GLM4_5_air,
    "glm-4.5-air"
);
impl_message_binding!(GLM4_5_air, TextMessage);
impl_model_markers!(GLM4_5_air: Chat, AsyncChat, ThinkEnable);

define_model_type!(
    #[allow(non_camel_case_types)]
    GLM4_5_airx,
    "glm-4.5-airx"
);
impl_message_binding!(GLM4_5_airx, TextMessage);
impl_model_markers!(GLM4_5_airx: Chat, AsyncChat, ThinkEnable);

define_model_type!(
    #[allow(non_camel_case_types)]
    GLM4_flash_250414,
    "glm-4-flash-250414"
);
impl_message_binding!(GLM4_flash_250414, TextMessage);
impl_model_markers!(GLM4_flash_250414: Chat, AsyncChat);

define_model_type!(
    #[allow(non_camel_case_types)]
    GLM4_flashx_250414,
    "glm-4-flashx-250414"
);
impl_message_binding!(GLM4_flashx_250414, TextMessage);
impl_model_markers!(GLM4_flashx_250414: Chat, AsyncChat);

define_model_type!(
    #[allow(non_camel_case_types)]
    autoglm_phone,
    "autoglm-phone"
);
impl_message_binding!(autoglm_phone, VisionMessage);
impl_model_markers!(autoglm_phone: Chat);

define_model_type!(
    #[allow(non_camel_case_types)]
    GLM4_6v,
    "glm-4.6v"
);
impl_message_binding!(GLM4_6v, VisionMessage);
impl_model_markers!(GLM4_6v: Chat, AsyncChat);

define_model_type!(
    #[allow(non_camel_case_types)]
    GLM4_6v_flash,
    "glm-4.6v-flash"
);
impl_message_binding!(GLM4_6v_flash, VisionMessage);
impl_model_markers!(GLM4_6v_flash: Chat, AsyncChat);

define_model_type!(
    #[allow(non_camel_case_types)]
    GLM4_6v_flashx,
    "glm-4.6v-flashx"
);
impl_message_binding!(GLM4_6v_flashx, VisionMessage);
impl_model_markers!(GLM4_6v_flashx: Chat, AsyncChat);

define_model_type!(
    #[allow(non_camel_case_types)]
    GLM4v_flash,
    "glm-4v-flash"
);
impl_message_binding!(GLM4v_flash, VisionMessage);
impl_model_markers!(GLM4v_flash: Chat, AsyncChat);

define_model_type!(
    #[allow(non_camel_case_types)]
    GLM4_1v_thinking_flash,
    "glm-4.1v-thinking-flash"
);
impl_message_binding!(GLM4_1v_thinking_flash, VisionMessage);
impl_model_markers!(GLM4_1v_thinking_flash: Chat, AsyncChat, ThinkEnable);

define_model_type!(
    #[allow(non_camel_case_types)]
    GLM4_1v_thinking_flashx,
    "glm-4.1v-thinking-flashx"
);
impl_message_binding!(GLM4_1v_thinking_flashx, VisionMessage);
impl_model_markers!(GLM4_1v_thinking_flashx: Chat, AsyncChat, ThinkEnable);

define_model_type!(
    #[allow(non_camel_case_types)]
    GLM4_voice,
    "glm-4-voice"
);
impl_message_binding!(GLM4_voice, VoiceMessage);
impl_model_markers!(GLM4_voice: Chat, AsyncChat);

macro_rules! impl_text_request_schema {
    ($($model:ty),+ $(,)?) => {
        $(
            impl ChatRequestModel for $model {
                const MAX_TOKENS: u32 = 131_072;
            }

            impl ChatToolSupport for $model {
                type Tool = Tools;
            }

            impl ResponseFormatEnable for $model {}
        )+
    };
}

macro_rules! impl_vision_request_schema {
    ($($model:ty),+ $(,)?) => {
        $(
            impl ChatRequestModel for $model {
                const MAX_TOKENS: u32 = 131_072;
            }

            impl ChatToolSupport for $model {
                type Tool = Function;
            }
        )+
    };
}

impl_text_request_schema!(
    GLM5_2,
    GLM5_1,
    GLM5_1_highspeed,
    GLM5_turbo,
    GLM5,
    GLM4_7,
    GLM4_7_flash,
    GLM4_7_flashx,
    GLM4_6,
    GLM4_5_flash,
    GLM4_5_air,
    GLM4_5_airx,
    GLM4_flash_250414,
    GLM4_flashx_250414,
);

impl_vision_request_schema!(
    GLM5V_turbo,
    autoglm_phone,
    GLM4_6v,
    GLM4_6v_flash,
    GLM4_6v_flashx,
    GLM4v_flash,
    GLM4_1v_thinking_flash,
    GLM4_1v_thinking_flashx,
);

impl ChatRequestModel for GLM4_voice {
    const MAX_TOKENS: u32 = 4_096;
}

impl WatermarkEnable for GLM4_voice {}

define_model_type!(
    #[allow(non_camel_case_types)]
    GLM_realtime_flash,
    "glm-realtime-flash"
);

define_model_type!(
    #[allow(non_camel_case_types)]
    GLM_realtime_air,
    "glm-realtime-air"
);

#[cfg(test)]
mod tests {
    use super::*;

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
    fn request_schema_capabilities_are_typed_by_model_family() {
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
        assert_eq!(GLM5_2::MAX_TOKENS, 131_072);
        assert_eq!(GLM4_voice::MAX_TOKENS, 4_096);
    }

    #[test]
    fn official_chat_model_names_match_snapshot() {
        let models = [
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
    fn official_realtime_model_names_match_asyncapi() {
        assert_eq!(String::from(GLM_realtime_flash {}), "glm-realtime-flash");
        assert_eq!(String::from(GLM_realtime_air {}), "glm-realtime-air");
    }

    #[test]
    fn capability_markers_cover_the_frozen_sync_and_async_enums() {
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
