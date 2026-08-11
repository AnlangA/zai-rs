use serde::{Deserialize, Serialize};
use validator::Validate;

/// Query parameters for listing voices.
#[derive(Debug, Clone, Serialize, Validate)]
#[validate(schema(function = "validate_voice_list_query"))]
pub struct VoiceListQuery {
    /// Voice-name filter. Pass the unescaped value; the client percent-encodes
    /// query parameters.
    #[serde(rename = "voiceName", skip_serializing_if = "Option::is_none")]
    #[validate(length(min = 1))]
    pub voice_name: Option<String>,
    /// Voice origin (`OFFICIAL` or `PRIVATE` on the wire).
    #[serde(rename = "voiceType", skip_serializing_if = "Option::is_none")]
    pub voice_type: Option<VoiceType>,
}

fn validate_voice_list_query(query: &VoiceListQuery) -> Result<(), validator::ValidationError> {
    if query
        .voice_name
        .as_deref()
        .is_some_and(|name| name.trim().is_empty())
    {
        Err(validator::ValidationError::new(
            "voice_name_must_not_be_blank",
        ))
    } else {
        Ok(())
    }
}

impl Default for VoiceListQuery {
    fn default() -> Self {
        Self::new()
    }
}

impl VoiceListQuery {
    /// Create a new empty voice-list query.
    pub fn new() -> Self {
        Self {
            voice_name: None,
            voice_type: None,
        }
    }
    /// Filter by voice name.
    pub fn with_voice_name(mut self, name: impl Into<String>) -> Self {
        self.voice_name = Some(name.into());
        self
    }
    /// Filter by voice type (official / private).
    pub fn with_voice_type(mut self, vt: VoiceType) -> Self {
        self.voice_type = Some(vt);
        self
    }
}

/// Voice origin: official preset or user-cloned private voice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum VoiceType {
    /// Official preset voice.
    Official,
    /// User-cloned private voice.
    Private,
}

impl VoiceType {
    /// Return the canonical upstream string (`"OFFICIAL"` / `"PRIVATE"`).
    pub fn as_str(&self) -> &'static str {
        match self {
            VoiceType::Official => "OFFICIAL",
            VoiceType::Private => "PRIVATE",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_query_serialization_uses_upstream_camel_case_names() {
        assert_eq!(
            serde_json::to_value(VoiceListQuery::new()).unwrap(),
            serde_json::json!({})
        );

        let value = serde_json::to_value(
            VoiceListQuery::new()
                .with_voice_name("中文 &/?=+")
                .with_voice_type(VoiceType::Private),
        )
        .unwrap();
        assert_eq!(
            value,
            serde_json::json!({
                "voiceName": "中文 &/?=+",
                "voiceType": "PRIVATE"
            })
        );
        assert!(value.get("voice_name").is_none());
        assert!(value.get("voice_type").is_none());
    }
}
