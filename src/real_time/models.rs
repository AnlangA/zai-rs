//! # Real-time Model Definitions
//!
//! This module contains model definitions for the GLM-Realtime API.
//! It defines the available real-time models and their capabilities.

use crate::model::traits::{Model, ModelTrait};
use serde::{Deserialize, Serialize};

/// Real-time model base trait
pub trait RealtimeModel: Model {
    /// Returns the websocket URL for the real-time API
    fn websocket_url(&self) -> String {
        "wss://open.bigmodel.cn/api/paas/v4/realtime".to_string()
    }
}

/// GLM-Realtime-Flash model - 9B parameter model for real-time audio/video conversations
///
/// Capabilities:
/// - Real-time audio and video conversations
/// - Function calling support
/// - Voice Activity Detection
/// - Audio processing with noise reduction
///
/// Pricing:
/// - Audio: 0.18元/分钟
/// - Video: 1.2元/分钟
///
/// Context Window:
/// - Audio: 8K tokens (approximately 20 rounds of conversation)
/// - Video: 32K tokens
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GLMRealtimeFlash;

impl ModelTrait for GLMRealtimeFlash {
    fn model(&self) -> &'static str {
        "glm-realtime-flash"
    }
}

impl RealtimeModel for GLMRealtimeFlash {}

/// GLM-Realtime-Air model - 32B parameter model for real-time audio/video conversations
///
/// Capabilities:
/// - Real-time audio and video conversations
/// - Function calling support
/// - Voice Activity Detection
/// - Audio processing with noise reduction
///
/// Pricing:
/// - Audio: 0.3元/分钟
/// - Video: 2.1元/分钟
///
/// Context Window:
/// - Audio: 8K tokens (approximately 20 rounds of conversation)
/// - Video: 32K tokens
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GLMRealtimeAir;

impl ModelTrait for GLMRealtimeAir {
    fn model(&self) -> &'static str {
        "glm-realtime-air"
    }
}

impl RealtimeModel for GLMRealtimeAir {}
