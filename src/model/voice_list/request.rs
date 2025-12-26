use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Clone, Serialize, Validate)]
pub struct VoiceListQuery {
    /// 音色名称, 如果传入中文, 需要 url encode
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice_name: Option<String>,
    /// 音色类型: OFFICIAL / PRIVATE
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice_type: Option<VoiceType>,
}

impl Default for VoiceListQuery {
    fn default() -> Self {
        Self::new()
    }
}

impl VoiceListQuery {
    pub fn new() -> Self {
        Self {
            voice_name: None,
            voice_type: None,
        }
    }
    pub fn with_voice_name(mut self, name: impl Into<String>) -> Self {
        self.voice_name = Some(name.into());
        self
    }
    pub fn with_voice_type(mut self, vt: VoiceType) -> Self {
        self.voice_type = Some(vt);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum VoiceType {
    Official,
    Private,
}

impl VoiceType {
    pub fn as_str(&self) -> &'static str {
        match self {
            VoiceType::Official => "OFFICIAL",
            VoiceType::Private => "PRIVATE",
        }
    }
}
