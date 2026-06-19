use serde::{Deserialize, Serialize};
use validator::Validate;

use super::request::VoiceType;

/// Top-level response from the voice-list endpoint.
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct VoiceListResponse {
    /// Matching voices (may be empty).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice_list: Option<Vec<VoiceItem>>,
}

/// A single voice entry in a [`VoiceListResponse`].
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct VoiceItem {
    /// Voice id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice: Option<String>,
    /// Human-readable voice name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice_name: Option<String>,
    /// Voice origin (official / private).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice_type: Option<VoiceType>,
    /// Download URL for a sample of the voice.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub download_url: Option<String>,
    /// Creation timestamp of the voice.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub create_time: Option<String>,
}
