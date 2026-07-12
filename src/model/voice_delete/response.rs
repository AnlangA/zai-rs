use serde::{Deserialize, Deserializer, Serialize, de::Error as _};
use validator::Validate;

/// Response from the voice-delete endpoint.
#[derive(Debug, Clone, Serialize, Validate)]
pub struct VoiceDeleteResponse {
    /// Id of the deleted voice.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice: Option<String>,
    /// Timestamp at which the deletion took effect.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub update_time: Option<String>,
}

#[derive(Deserialize)]
struct VoiceDeleteResponseWire {
    voice: Option<String>,
    update_time: Option<String>,
}

impl<'de> Deserialize<'de> for VoiceDeleteResponse {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = VoiceDeleteResponseWire::deserialize(deserializer)?;
        if wire.voice.is_none() && wire.update_time.is_none() {
            return Err(D::Error::custom(
                "voice-delete response contained no documented fields",
            ));
        }
        Ok(Self {
            voice: wire.voice,
            update_time: wire.update_time,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn response_requires_one_documented_non_null_field() {
        assert!(serde_json::from_str::<VoiceDeleteResponse>("{}").is_err());
        assert!(serde_json::from_str::<VoiceDeleteResponse>(r#"{"voice":null}"#).is_err());
        assert!(serde_json::from_str::<VoiceDeleteResponse>(r#"{"update_time":"now"}"#).is_ok());
    }
}
