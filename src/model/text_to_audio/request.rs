use serde::Serialize;
use validator::Validate;

use super::super::traits::*;

/// Request body for text-to-speech synthesis.
#[derive(Debug, Clone, Serialize, Validate)]
pub struct TextToAudioBody<N>
where
    N: ModelName + TextToAudio + Serialize,
{
    /// TTS model (e.g., cogtts)
    pub model: N,

    /// Text to convert to speech (at most 1,024 Unicode scalar values).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(length(max = 1024))]
    pub input: Option<String>,

    /// Voice preset
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice: Option<Voice>,

    /// Speed in [0.5, 2]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(range(min = 0.5, max = 2.0))]
    pub speed: Option<f32>,

    /// Volume in (0, 10] — strictly greater than 0. The validator
    /// `range` cannot express a strict lower bound, so a `0.0` volume is
    /// rejected by a dedicated check in the request's `validate()`/builder.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(range(min = 0.0, max = 10.0))]
    pub volume: Option<f32>,

    /// Output audio format
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<TtsAudioFormat>,

    /// Watermark toggle
    #[serde(skip_serializing_if = "Option::is_none")]
    pub watermark_enabled: Option<bool>,
}

impl<N> TextToAudioBody<N>
where
    N: ModelName + TextToAudio + Serialize,
{
    /// Create a new TTS request body for the given model (defaults: voice
    /// `Tongtong`, format `Wav`).
    pub fn new(model: N) -> Self {
        Self {
            model,
            input: None,
            voice: Some(Voice::Tongtong),
            speed: None,
            volume: None,
            response_format: Some(TtsAudioFormat::Wav),
            watermark_enabled: None,
        }
    }

    /// Set the input text to synthesize.
    pub fn with_input(mut self, input: impl Into<String>) -> Self {
        self.input = Some(input.into());
        self
    }

    /// Set the voice preset.
    pub fn with_voice(mut self, voice: Voice) -> Self {
        self.voice = Some(voice);
        self
    }

    /// Set the playback speed (`0.5`–`2.0`).
    pub fn with_speed(mut self, speed: f32) -> Self {
        self.speed = Some(speed);
        self
    }

    /// Set the playback volume (greater than `0.0` and at most `10.0`).
    pub fn with_volume(mut self, volume: f32) -> Self {
        self.volume = Some(volume);
        self
    }

    /// Set the output audio format.
    pub fn with_response_format(mut self, fmt: TtsAudioFormat) -> Self {
        self.response_format = Some(fmt);
        self
    }

    /// Enable/disable the audio watermark.
    pub fn with_watermark_enabled(mut self, enabled: bool) -> Self {
        self.watermark_enabled = Some(enabled);
        self
    }
}

/// Built-in TTS voice presets.
#[derive(Debug, Clone)]
pub enum Voice {
    /// Tongtong voice.
    Tongtong,
    /// Chuichui voice.
    Chuichui,
    /// Xiaochen voice.
    Xiaochen,
    /// Jam voice.
    Jam,
    /// Kazi voice.
    Kazi,
    /// Douji voice.
    Douji,
    /// Luodo voice.
    Luodo,
}

impl serde::Serialize for Voice {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let s = match self {
            Voice::Tongtong => "tongtong",
            Voice::Chuichui => "chuichui",
            Voice::Xiaochen => "xiaochen",
            Voice::Jam => "jam",
            Voice::Kazi => "kazi",
            Voice::Douji => "douji",
            Voice::Luodo => "luodo",
        };
        serializer.serialize_str(s)
    }
}

/// Supported output audio formats for TTS.
#[derive(Debug, Clone)]
pub enum TtsAudioFormat {
    /// PCM WAV container.
    Wav,
}

impl serde::Serialize for TtsAudioFormat {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let s = match self {
            TtsAudioFormat::Wav => "wav",
        };
        serializer.serialize_str(s)
    }
}
