use serde::{Deserialize, Serialize};
use validator::Validate;

/// Response from the voice-delete endpoint.
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct VoiceDeleteResponse {
    /// Id of the deleted voice.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice: Option<String>,
    /// Timestamp at which the deletion took effect.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub update_time: Option<String>,
}
