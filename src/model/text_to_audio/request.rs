use super::super::traits::*;
use serde::Serialize;
use validator::Validate;

#[derive(Debug, Clone, Serialize, Validate)]
pub struct TextToAudioBody<N>
where
    N: ModelName + TextToAudio + Serialize,
{
    /// TTS model (e.g., cogtts)
    pub model: N,

    /// Text to convert to speech (max 4096)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(length(max = 4096))]
    pub input: Option<String>,

    /// Voice preset
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice: Option<Voice>,

    /// Speed in [0.5, 2]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(range(min = 0.5, max = 2.0))]
    pub speed: Option<f32>,

    /// Volume in (0, 10]; we validate as [0.0, 10.0] for simplicity
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

    pub fn with_input(mut self, input: impl Into<String>) -> Self {
        self.input = Some(input.into());
        self
    }

    pub fn with_voice(mut self, voice: Voice) -> Self {
        self.voice = Some(voice);
        self
    }

    pub fn with_speed(mut self, speed: f32) -> Self {
        self.speed = Some(speed);
        self
    }

    pub fn with_volume(mut self, volume: f32) -> Self {
        self.volume = Some(volume);
        self
    }

    pub fn with_response_format(mut self, fmt: TtsAudioFormat) -> Self {
        self.response_format = Some(fmt);
        self
    }

    pub fn with_watermark_enabled(mut self, enabled: bool) -> Self {
        self.watermark_enabled = Some(enabled);
        self
    }
}

#[derive(Debug, Clone)]
pub enum Voice {
    Tongtong,
    Chuichui,
    Xiaochen,
    Jam,
    Kazi,
    Douji,
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

#[derive(Debug, Clone)]
pub enum TtsAudioFormat {
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
