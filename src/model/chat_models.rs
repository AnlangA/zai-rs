//! # AI Model Type Definitions
//!
//! Defines all available AI model types for the Zhipu AI API, together with
//! their capability markers and message-type bindings.
//!
//! # Model Categories
//!
//! ## Text Models
//!
//! | Model | Struct | Thinking | Async | ToolStream |
//! |-------|--------|----------|-------|------------|
//! | glm-5.1 | [`GLM5_1`] | yes | yes | yes |
//! | glm-5 | [`GLM5`] | yes | yes | yes |
//! | glm-5-turbo | [`GLM5_turbo`] | yes | yes | yes |
//! | glm-4.7 | [`GLM4_7`] | yes | yes | yes |
//! | glm-4.7-flash | [`GLM4_7_flash`] | yes | yes | no |
//! | glm-4.7-flashx | [`GLM4_7_flashx`] | yes | yes | no |
//! | glm-4.6 | [`GLM4_6`] | yes | yes | yes |
//! | glm-4.5 | [`GLM4_5`] | yes | yes | no |
//! | glm-4.5-X | [`GLM4_5_x`] | yes | yes | no |
//! | glm-4.5-flash | [`GLM4_5_flash`] | yes | yes | no |
//! | glm-4.5-air | [`GLM4_5_air`] | yes | yes | no |
//! | glm-4.5-airx | [`GLM4_5_airx`] | yes | yes | no |
//!
//! ## Vision Models
//!
//! | Model | Struct | Message Type |
//! |-------|--------|--------------|
//! | autoglm-phone | [`autoglm_phone`] | [`VisionMessage`](super::chat_message_types::VisionMessage) |
//! | glm-4.6v | [`GLM4_6v`] | [`VisionMessage`] |
//! | glm-4.6v-flash | [`GLM4_6v_flash`] | [`VisionMessage`] |
//! | glm-4.6v-flashx | [`GLM4_6v_flashx`] | [`VisionMessage`] |
//! | glm-4.5v | [`GLM4_5v`] | [`VisionMessage`] |
//!
//! ## Voice Models
//!
//! | Model | Struct | Message Type |
//! |-------|--------|--------------|
//! | glm-4-voice | [`GLM4_voice`] | [`VoiceMessage`](super::chat_message_types::VoiceMessage) |
//!
//! # Usage
//!
//! ```rust,ignore
//! use zai_rs::model::chat_models::*;
//! use zai_rs::model::chat_message_types::TextMessage;
//! use zai_rs::model::chat::data::ChatCompletion;
//!
//! let model = GLM5_turbo {};
//! let messages = TextMessage::user("Hello");
//! let client = ChatCompletion::new(model, messages, api_key);
//! ```
//!
//! # Defining New Models
//!
//! Use the [`define_model_type!`](crate::define_model_type) macro to create
//! model structs, [`impl_message_binding!`](crate::impl_message_binding) to
//! bind compatible message types, and
//! [`impl_model_markers!`](crate::impl_model_markers) to declare capabilities.

use super::traits::*;
use crate::{
    define_model_type, impl_message_binding, impl_model_markers,
    model::chat_message_types::{TextMessage, VisionMessage, VoiceMessage},
};

// ============================================================================
// Text Models
// ============================================================================

define_model_type!(GLM5_1, "glm-5.1");
impl_message_binding!(GLM5_1, TextMessage);
impl_model_markers!(GLM5_1: Chat, AsyncChat, ThinkEnable, ToolStreamEnable);

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

define_model_type!(GLM4_7, "glm-4.7");
impl_message_binding!(GLM4_7, TextMessage);
impl_model_markers!(GLM4_7: Chat, AsyncChat, ThinkEnable, ToolStreamEnable);

define_model_type!(
    #[allow(non_camel_case_types)]
    GLM4_7_flash,
    "glm-4.7-flash"
);
impl_message_binding!(GLM4_7_flash, TextMessage);
impl_model_markers!(GLM4_7_flash: Chat, AsyncChat, ThinkEnable);

define_model_type!(
    #[allow(non_camel_case_types)]
    GLM4_7_flashx,
    "glm-4.7-flashx"
);
impl_message_binding!(GLM4_7_flashx, TextMessage);
impl_model_markers!(GLM4_7_flashx: Chat, AsyncChat, ThinkEnable);

define_model_type!(GLM4_6, "glm-4.6");
impl_message_binding!(GLM4_6, TextMessage);
impl_model_markers!(GLM4_6: Chat, AsyncChat, ThinkEnable);
impl ToolStreamEnable for GLM4_6 {}

define_model_type!(GLM4_5, "glm-4.5");
impl_message_binding!(GLM4_5, TextMessage);
impl_model_markers!(GLM4_5: Chat, AsyncChat, ThinkEnable);

define_model_type!(
    #[allow(non_camel_case_types)]
    GLM4_5_x,
    "glm-4.5-X"
);
impl_message_binding!(GLM4_5_x, TextMessage);
impl_model_markers!(GLM4_5_x: Chat, AsyncChat, ThinkEnable);

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

// ============================================================================
// Multimodal Models - Vision
// ============================================================================

define_model_type!(
    #[allow(non_camel_case_types)]
    autoglm_phone,
    "autoglm-phone"
);
impl_message_binding!(autoglm_phone, VisionMessage);
impl_model_markers!(autoglm_phone: Chat, AsyncChat);

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
    GLM4_5v,
    "glm-4.5v"
);
impl_message_binding!(GLM4_5v, VisionMessage);
impl_model_markers!(GLM4_5v: Chat, AsyncChat);

// ============================================================================
// Multimodal Models - Voice
// ============================================================================

define_model_type!(
    #[allow(non_camel_case_types)]
    GLM4_voice,
    "glm-4-voice"
);
impl_message_binding!(GLM4_voice, VoiceMessage);
impl_model_markers!(GLM4_voice: Chat, AsyncChat);
