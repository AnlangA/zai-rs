use serde::{Deserialize, Serialize};
use validator::Validate;

/// Response from the voice-clone endpoint.
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct VoiceCloneResponse {
    /// Registered voice id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice: Option<String>,
    /// File id of the generated preview audio.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_id: Option<String>,
    /// File purpose (expected fixed value: "voice-clone-output").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_purpose: Option<String>, // expected fixed value: "voice-clone-output"
    /// Client-side request id, if one was supplied.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
}
