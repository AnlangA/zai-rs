use serde::Serialize;
use validator::Validate;

use super::super::traits::*;

/// Request body for text-to-speech synthesis.
#[derive(Clone, Serialize, Validate)]
#[validate(schema(function = "validate_tts_body"))]
pub struct TextToAudioBody<N>
where
    N: TextToAudio,
{
    /// TTS model (for example, `glm-tts`).
    pub(super) model: N,

    /// Text to convert to speech (at most 1,024 Unicode scalar values).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) input: Option<String>,

    /// Built-in preset or cloned voice identifier.
    pub(super) voice: Voice,

    /// Speed in [0.5, 2]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(range(min = 0.5, max = 2.0))]
    pub(super) speed: Option<f32>,

    /// Volume in `(0, 10]`. A schema-level validator enforces the strict lower
    /// bound and rejects non-finite values.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(range(min = 0.0, max = 10.0))]
    pub(super) volume: Option<f32>,

    /// Output audio format
    pub(super) response_format: TtsAudioFormat,

    /// Whether the endpoint must return SSE audio chunks.
    pub(super) stream: bool,

    /// Encoding used inside SSE `data:` payloads. Valid only for streaming.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) encode_format: Option<TtsEncodeFormat>,

    /// Watermark toggle
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) watermark_enabled: Option<bool>,
}

impl<N> std::fmt::Debug for TextToAudioBody<N>
where
    N: TextToAudio + std::fmt::Debug,
{
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let voice = match &self.voice {
            Voice::Tongtong => "tongtong",
            Voice::Chuichui => "chuichui",
            Voice::Xiaochen => "xiaochen",
            Voice::Jam => "jam",
            Voice::Kazi => "kazi",
            Voice::Douji => "douji",
            Voice::Luodo => "luodo",
            Voice::Cloned(_) => "[CLONED]",
        };
        formatter
            .debug_struct("TextToAudioBody")
            .field("model", &self.model)
            .field("input_configured", &self.input.is_some())
            .field("voice", &voice)
            .field("speed_configured", &self.speed.is_some())
            .field("volume_configured", &self.volume.is_some())
            .field("response_format", &self.response_format)
            .field("stream", &self.stream)
            .field("encode_format", &self.encode_format)
            .field("watermark_enabled", &self.watermark_enabled)
            .finish()
    }
}

fn validate_tts_body<N>(body: &TextToAudioBody<N>) -> Result<(), validator::ValidationError>
where
    N: TextToAudio,
{
    let Some(input) = body.input.as_deref() else {
        return Err(validator::ValidationError::new("input_required"));
    };
    let input_len = input.chars().count();
    if input.trim().is_empty() || !(1..=1024).contains(&input_len) {
        return Err(validator::ValidationError::new("input_length"));
    }
    if !body.voice.is_valid() {
        return Err(validator::ValidationError::new("voice_required"));
    }
    if body.speed.is_some_and(|speed| !speed.is_finite()) {
        return Err(validator::ValidationError::new("speed_must_be_finite"));
    }
    if body
        .volume
        .is_some_and(|volume| !volume.is_finite() || volume <= 0.0)
    {
        return Err(validator::ValidationError::new("volume_must_be_positive"));
    }
    if body.stream {
        if body.response_format != TtsAudioFormat::Pcm {
            return Err(validator::ValidationError::new("stream_requires_pcm"));
        }
        if body.encode_format.is_none() {
            return Err(validator::ValidationError::new(
                "stream_requires_encode_format",
            ));
        }
    } else if body.encode_format.is_some() {
        return Err(validator::ValidationError::new(
            "encode_format_is_stream_only",
        ));
    }
    Ok(())
}

impl<N> TextToAudioBody<N>
where
    N: TextToAudio,
{
    /// Create a new TTS request body for the given model.
    ///
    /// The built-in `tongtong` voice and documented PCM format are selected.
    pub fn new(model: N) -> Self {
        Self {
            model,
            input: None,
            voice: Voice::Tongtong,
            speed: None,
            volume: None,
            response_format: TtsAudioFormat::Pcm,
            stream: false,
            encode_format: None,
            watermark_enabled: None,
        }
    }

    /// Borrow the selected model marker.
    pub fn model(&self) -> &N {
        &self.model
    }

    /// Borrow the required input text, when configured.
    pub fn input(&self) -> Option<&str> {
        self.input.as_deref()
    }

    /// Borrow the selected voice.
    pub fn voice(&self) -> &Voice {
        &self.voice
    }

    /// Configured speed, or `None` to use the service default.
    pub fn speed(&self) -> Option<f32> {
        self.speed
    }

    /// Configured volume, or `None` to use the service default.
    pub fn volume(&self) -> Option<f32> {
        self.volume
    }

    /// Selected output format.
    pub fn response_format(&self) -> TtsAudioFormat {
        self.response_format
    }

    /// Whether this body requests SSE audio chunks.
    pub fn is_streaming(&self) -> bool {
        self.stream
    }

    /// Streaming payload encoding, when streaming is enabled.
    pub fn encode_format(&self) -> Option<TtsEncodeFormat> {
        self.encode_format
    }

    /// Optional watermark setting.
    pub fn watermark_enabled(&self) -> Option<bool> {
        self.watermark_enabled
    }

    /// Set the input text to synthesize.
    pub fn with_input(mut self, input: impl Into<String>) -> Self {
        self.input = Some(input.into());
        self
    }

    /// Set the voice preset.
    pub fn with_voice(mut self, voice: Voice) -> Self {
        self.voice = voice;
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
        self.response_format = fmt;
        self
    }

    /// Enable/disable the audio watermark.
    pub fn with_watermark_enabled(mut self, enabled: bool) -> Self {
        self.watermark_enabled = Some(enabled);
        self
    }

    pub(super) fn with_stream(mut self, stream: bool) -> Self {
        self.stream = stream;
        if stream {
            self.response_format = TtsAudioFormat::Pcm;
            self.encode_format.get_or_insert(TtsEncodeFormat::Base64);
        } else {
            self.encode_format = None;
        }
        self
    }

    pub(super) fn with_encode_format(mut self, format: TtsEncodeFormat) -> Self {
        self.encode_format = Some(format);
        self
    }
}

/// A built-in TTS voice or an identifier returned by voice cloning.
#[derive(Clone, PartialEq, Eq)]
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
    /// Voice identifier returned by the voice-clone API.
    Cloned(String),
}

impl std::fmt::Debug for Voice {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Tongtong => formatter.write_str("Tongtong"),
            Self::Chuichui => formatter.write_str("Chuichui"),
            Self::Xiaochen => formatter.write_str("Xiaochen"),
            Self::Jam => formatter.write_str("Jam"),
            Self::Kazi => formatter.write_str("Kazi"),
            Self::Douji => formatter.write_str("Douji"),
            Self::Luodo => formatter.write_str("Luodo"),
            Self::Cloned(_) => formatter.write_str("Cloned([REDACTED])"),
        }
    }
}

impl Voice {
    /// Construct a cloned voice identifier, rejecting blank values.
    pub fn cloned(id: impl Into<String>) -> crate::ZaiResult<Self> {
        let id = id.into();
        if id.trim().is_empty() || id.trim() != id {
            return Err(crate::ZaiError::ApiError {
                code: crate::client::error::codes::SDK_VALIDATION,
                message: "cloned voice id must be non-blank without surrounding whitespace"
                    .to_string(),
            });
        }
        Ok(Self::Cloned(id))
    }

    /// Wire identifier used by the TTS endpoint.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Tongtong => "tongtong",
            Self::Chuichui => "chuichui",
            Self::Xiaochen => "xiaochen",
            Self::Jam => "jam",
            Self::Kazi => "kazi",
            Self::Douji => "douji",
            Self::Luodo => "luodo",
            Self::Cloned(id) => id,
        }
    }

    fn is_valid(&self) -> bool {
        let id = self.as_str();
        !id.is_empty() && id.trim() == id
    }
}

impl Serialize for Voice {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

/// Supported output audio formats for TTS.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TtsAudioFormat {
    /// WAV container.
    Wav,
    /// Headerless PCM audio (the SDK and service default).
    Pcm,
}

/// Encoding used for audio bytes inside streaming SSE `data:` payloads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TtsEncodeFormat {
    /// Standard padded base64.
    Base64,
    /// Lowercase or uppercase hexadecimal text.
    Hex,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::text_to_audio::GlmTts;

    #[test]
    fn validation_requires_non_blank_input() {
        assert!(TextToAudioBody::new(GlmTts {}).validate().is_err());
        assert!(
            TextToAudioBody::new(GlmTts {})
                .with_input("   ")
                .validate()
                .is_err()
        );

        let invalid = TextToAudioBody::new(GlmTts {})
            .with_input("hello")
            .with_voice(Voice::Cloned(String::new()));
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn validation_rejects_zero_and_non_finite_controls() {
        assert!(
            TextToAudioBody::new(GlmTts {})
                .with_input("hello")
                .with_volume(0.0)
                .validate()
                .is_err()
        );
        assert!(
            TextToAudioBody::new(GlmTts {})
                .with_input("hello")
                .with_speed(f32::NAN)
                .validate()
                .is_err()
        );
    }

    #[test]
    fn default_format_is_explicit_pcm_and_cloned_voice_is_a_plain_string() {
        let default = TextToAudioBody::new(GlmTts {}).with_input("hello");
        let json = serde_json::to_value(default).unwrap();
        assert_eq!(json["voice"], "tongtong");
        assert_eq!(json["response_format"], "pcm");
        assert_eq!(json["stream"], false);
        assert!(json.get("encode_format").is_none());

        let cloned = Voice::cloned("voice-clone-123").unwrap();
        let body = TextToAudioBody::new(GlmTts {})
            .with_input("hello")
            .with_voice(cloned);
        let json = serde_json::to_value(body).unwrap();
        assert_eq!(json["voice"], "voice-clone-123");
        assert!(Voice::cloned("  ").is_err());
        assert!(Voice::cloned(" voice-clone-123 ").is_err());

        let invalid = TextToAudioBody::new(GlmTts {})
            .with_input("hello")
            .with_voice(Voice::Cloned(String::new()));
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn streaming_body_forces_pcm_and_has_stream_only_encoding() {
        let body = TextToAudioBody::new(GlmTts {})
            .with_input("hello")
            .with_response_format(TtsAudioFormat::Wav)
            .with_stream(true)
            .with_encode_format(TtsEncodeFormat::Hex);
        assert!(body.validate().is_ok());
        let json = serde_json::to_value(body).unwrap();
        assert_eq!(json["stream"], true);
        assert_eq!(json["response_format"], "pcm");
        assert_eq!(json["encode_format"], "hex");
    }

    #[test]
    fn input_limit_counts_unicode_scalar_values() {
        assert!(
            TextToAudioBody::new(GlmTts {})
                .with_input("你".repeat(1024))
                .validate()
                .is_ok()
        );
        assert!(
            TextToAudioBody::new(GlmTts {})
                .with_input("你".repeat(1025))
                .validate()
                .is_err()
        );
    }

    #[test]
    fn debug_redacts_input_and_cloned_voice_id() {
        let body = TextToAudioBody::new(GlmTts {})
            .with_input("private speech")
            .with_voice(Voice::cloned("private-voice-id").unwrap());
        let debug = format!("{body:?}");
        assert!(!debug.contains("private speech"));
        assert!(!debug.contains("private-voice-id"));
        assert!(debug.contains("input_configured: true"));
        assert!(debug.contains("voice: \"[CLONED]\""));
    }
}
