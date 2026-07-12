use serde::{Deserialize, Serialize};
use validator::Validate;

/// Query parameters for listing voices.
#[derive(Debug, Clone, Serialize, Validate)]
pub struct VoiceListQuery {
    /// Voice-name filter. Pass the unescaped value; the client percent-encodes
    /// query parameters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice_name: Option<String>,
    /// Voice origin (`OFFICIAL` or `PRIVATE` on the wire).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice_type: Option<VoiceType>,
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
