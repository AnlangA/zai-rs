use serde::{Deserialize, Deserializer, Serialize, de::Error as _};
use validator::Validate;

/// Response from the voice-clone endpoint.
#[derive(Debug, Clone, Serialize, Validate)]
pub struct VoiceCloneResponse {
    /// Registered voice id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice: Option<String>,
    /// File id of the generated preview audio.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_id: Option<String>,
    /// File purpose (expected fixed value: "voice-clone-output").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_purpose: Option<String>,
    /// Client-side request id, if one was supplied.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
}

#[derive(Deserialize)]
struct VoiceCloneResponseWire {
    voice: Option<String>,
    file_id: Option<String>,
    file_purpose: Option<String>,
    request_id: Option<String>,
}

impl<'de> Deserialize<'de> for VoiceCloneResponse {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = VoiceCloneResponseWire::deserialize(deserializer)?;
        if wire.voice.is_none()
            && wire.file_id.is_none()
            && wire.file_purpose.is_none()
            && wire.request_id.is_none()
        {
            return Err(D::Error::custom(
                "voice-clone response contained no documented fields",
            ));
        }
        Ok(Self {
            voice: wire.voice,
            file_id: wire.file_id,
            file_purpose: wire.file_purpose,
            request_id: wire.request_id,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn response_requires_one_documented_non_null_field() {
        assert!(serde_json::from_str::<VoiceCloneResponse>("{}").is_err());
        assert!(serde_json::from_str::<VoiceCloneResponse>(r#"{"voice":null}"#).is_err());
        assert!(serde_json::from_str::<VoiceCloneResponse>(r#"{"voice":"voice-1"}"#).is_ok());
    }
}
