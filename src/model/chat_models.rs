//! # AI Model Type Definitions
//!
//! This module defines all available AI model types for the Zhipu AI API,
//! along with their capabilities, message type bindings, and trait implementations.
//!
//! ## Model Categories
//!
//! ### Text Models
//! - **GLM-4.5** - Advanced reasoning model with thinking capabilities
//! - **GLM-4.5-Flash** - Optimized for speed and efficiency
//! - **GLM-4.5-Air** - Lightweight model for cost-effective applications
//! - **GLM-4.5-X** - Extended capabilities model
//! - **GLM-4.5-AirX** - Ultra-lightweight variant
//!
//! ### Multimodal Models
//! - **GLM-4.5V** - Vision-enabled model supporting images and videos
//! - **GLM-4-Voice** - Voice-enabled model for audio interactions
//!
//! ## Model Capabilities
//!
//! | Model | Text | Vision | Voice | Thinking | Async |
//! |-------|------|--------|--------|----------|--------|
//! | GLM-4.5 | ✓ | ✗ | ✗ | ✓ | ✓ |
//! | GLM-4.5-Flash | ✓ | ✗ | ✗ | ✓ | ✓ |
//! | GLM-4.5-Air | ✓ | ✗ | ✗ | ✓ | ✓ |
//! | GLM-4.5-X | ✓ | ✗ | ✗ | ✓ | ✓ |
//! | GLM-4.5-AirX | ✓ | ✗ | ✗ | ✓ | ✓ |
//! | GLM-4.5V | ✓ | ✓ | ✗ | ✗ | ✓ |
//! | GLM-4-Voice | ✓ | ✗ | ✓ | ✗ | ✓ |
//!
//! ## Usage
//!
//! Models are used by creating instances and passing them to chat completion requests:
//!
//! ```rust,ignore
//! use zai_rs::model::chat_models::*;
//! use zai_rs::model::chat_message_types::*;
//! use zai_rs::model::chat::data::ChatCompletion;
//!
//! let model = GLM4_5_flash {};
//! let messages = TextMessage::user("Hello, how can you help me?");
//! let client = ChatCompletion::new(model, messages, api_key);
//! ```

use super::traits::*;
use crate::model::chat_message_types::{TextMessage, VisionMessage, VoiceMessage};
use crate::{define_model_type, impl_message_binding, impl_model_markers};

define_model_type!(GLM4_7, "glm-4.7");
impl_message_binding!(GLM4_7, TextMessage);
impl_model_markers!(GLM4_7: Chat, AsyncChat, ThinkEnable, ToolStreamEnable);

define_model_type!(GLM4_6, "glm-4.6");
impl_message_binding!(GLM4_6, TextMessage);
impl_model_markers!(GLM4_6: Chat, AsyncChat, ThinkEnable);
impl ToolStreamEnable for GLM4_6 {}

define_model_type!(GLM4_5, "glm-4.5");
impl_message_binding!(GLM4_5, TextMessage);
impl_model_markers!(GLM4_5: Chat, AsyncChat, ThinkEnable);

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
    GLM4_5_x,
    "glm-4.5-X"
);
impl_message_binding!(GLM4_5_x, TextMessage);
impl_model_markers!(GLM4_5_x: Chat, AsyncChat, ThinkEnable);

define_model_type!(
    #[allow(non_camel_case_types)]
    GLM4_5_airx,
    "glm-4.5-airx"
);
impl_message_binding!(GLM4_5_airx, TextMessage);
impl_model_markers!(GLM4_5_airx: Chat, AsyncChat, ThinkEnable);

define_model_type!(
    #[allow(non_camel_case_types)]
    GLM4_5v,
    "glm-4.5v"
);
impl_message_binding!(GLM4_5v, VisionMessage);
impl_model_markers!(GLM4_5v: Chat, AsyncChat);

define_model_type!(
    #[allow(non_camel_case_types)]
    GLM4_voice,
    "glm-4-voice"
);
impl_message_binding!(GLM4_voice, VoiceMessage);
impl_model_markers!(GLM4_voice: Chat, AsyncChat);
