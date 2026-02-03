//! Real-time API types

use serde::{Deserialize, Serialize};

/// Real-time API event types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum RealTimeEvent {
    /// Audio data event
    #[serde(rename = "audio")]
    Audio { data: Vec<u8>, format: AudioFormat },

    /// Text transcription event
    #[serde(rename = "text")]
    Text { content: String, is_final: bool },

    /// Session started event
    #[serde(rename = "session_started")]
    SessionStarted { session_id: String },

    /// Session ended event
    #[serde(rename = "session_ended")]
    SessionEnded { reason: String },

    /// Error event
    #[serde(rename = "error")]
    Error { code: u16, message: String },

    /// Status update event
    #[serde(rename = "status")]
    Status { state: SessionState },
}

/// Audio format enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AudioFormat {
    #[serde(rename = "wav")]
    Wav,

    #[serde(rename = "mp3")]
    Mp3,

    #[serde(rename = "pcm")]
    Pcm,

    #[serde(rename = "opus")]
    Opus,
}

/// Session state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionState {
    #[serde(rename = "connecting")]
    Connecting,

    #[serde(rename = "connected")]
    Connected,

    #[serde(rename = "listening")]
    Listening,

    #[serde(rename = "processing")]
    Processing,

    #[serde(rename = "speaking")]
    Speaking,

    #[serde(rename = "disconnected")]
    Disconnected,

    #[serde(rename = "error")]
    Error,
}

/// Session configuration
#[derive(Debug, Clone, Serialize)]
pub struct SessionConfig {
    /// Audio format for input/output
    pub audio_format: AudioFormat,

    /// Sample rate in Hz
    pub sample_rate: u32,

    /// Number of audio channels
    pub channels: u8,

    /// Enable automatic transcription
    pub enable_transcription: bool,

    /// Enable voice activity detection
    pub enable_vad: bool,

    /// Session timeout in seconds
    pub timeout_seconds: u32,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            audio_format: AudioFormat::Pcm,
            sample_rate: 16000,
            channels: 1,
            enable_transcription: true,
            enable_vad: true,
            timeout_seconds: 300,
        }
    }
}

/// Session statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStats {
    /// Session duration in seconds
    pub duration_seconds: u64,

    /// Number of audio packets sent
    pub packets_sent: u64,

    /// Number of audio packets received
    pub packets_received: u64,

    /// Total bytes sent
    pub bytes_sent: u64,

    /// Total bytes received
    pub bytes_received: u64,

    /// Number of transcriptions
    pub transcription_count: u64,
}
