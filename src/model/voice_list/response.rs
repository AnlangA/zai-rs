use serde::{Deserialize, Serialize};
use validator::Validate;

use super::request::VoiceType;

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct VoiceListResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice_list: Option<Vec<VoiceItem>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct VoiceItem {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice_type: Option<VoiceType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub download_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub create_time: Option<String>,
}
