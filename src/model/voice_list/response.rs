use serde::{Deserialize, Deserializer, Serialize, de::Error as _};
use validator::Validate;

/// Top-level response from the voice-list endpoint.
#[derive(Debug, Clone, Serialize, Validate)]
pub struct VoiceListResponse {
    /// Matching voices (may be empty).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice_list: Option<Vec<VoiceItem>>,
}

#[derive(Deserialize)]
struct VoiceListResponseWire {
    voice_list: Option<Vec<VoiceItem>>,
}

impl<'de> Deserialize<'de> for VoiceListResponse {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = VoiceListResponseWire::deserialize(deserializer)?;
        let Some(voice_list) = wire.voice_list else {
            return Err(D::Error::custom(
                "voice-list response contained no documented non-null fields",
            ));
        };
        Ok(Self {
            voice_list: Some(voice_list),
        })
    }
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
    pub voice_type: Option<String>,
    /// Download URL for a sample of the voice.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub download_url: Option<String>,
    /// Creation timestamp of the voice.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub create_time: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn response_requires_the_documented_non_null_list_field() {
        assert!(serde_json::from_str::<VoiceListResponse>("{}").is_err());
        assert!(serde_json::from_str::<VoiceListResponse>(r#"{"voice_list":null}"#).is_err());
        assert!(serde_json::from_str::<VoiceListResponse>(r#"{"voice_list":[]}"#).is_ok());
    }

    #[test]
    fn item_properties_remain_optional_strings() {
        let item: VoiceItem = serde_json::from_str(r#"{"voice_type":"FUTURE"}"#).unwrap();
        assert_eq!(item.voice_type.as_deref(), Some("FUTURE"));
    }
}
