use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct VoiceCloneResponse {
    #[serde(skip_serializing_if = "Option::is_none")] 
    pub voice: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")] 
    pub file_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")] 
    pub file_purpose: Option<String>, // expected fixed value: "voice-clone-output"
    #[serde(skip_serializing_if = "Option::is_none")] 
    pub request_id: Option<String>,
}

