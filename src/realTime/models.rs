//! Real-time API model definitions

use serde::{Deserialize, Serialize};

/// Available real-time models
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RealTimeModel {
    /// GLM-4 Voice model for audio interactions
    #[serde(rename = "glm-4-voice")]
    Glm4Voice,

    /// GLM-4.5 with voice capabilities
    #[serde(rename = "glm-4.5-voice")]
    Glm45Voice,
}

impl RealTimeModel {
    /// Get the model identifier string
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Glm4Voice => "glm-4-voice",
            Self::Glm45Voice => "glm-4.5-voice",
        }
    }

    /// Check if the model supports transcription
    pub fn supports_transcription(&self) -> bool {
        matches!(self, Self::Glm4Voice | Self::Glm45Voice)
    }

    /// Check if the model supports synthesis
    pub fn supports_synthesis(&self) -> bool {
        matches!(self, Self::Glm4Voice | Self::Glm45Voice)
    }
}

impl Default for RealTimeModel {
    fn default() -> Self {
        Self::Glm4Voice
    }
}

impl std::fmt::Display for RealTimeModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}
