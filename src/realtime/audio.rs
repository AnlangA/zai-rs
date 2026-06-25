//! Audio format + base64 helpers for the realtime API.
//!
//! The GLM-Realtime protocol exchanges audio as base64-encoded payloads:
//!
//! - **Input** (`input_audio_buffer.append`): a base64-encoded **WAV** file
//!   (`input_audio_format: "wav"` = 16 kHz, `"wav48"` = 48 kHz).
//! - **Output** (`response.audio.delta`): base64-encoded raw **PCM** (when
//!   `output_audio_format: "pcm"`) or **MP3** (`"mp3"`); decoded to bytes by
//!   the transport layer before being handed to the caller.

use base64::Engine;
use serde::{Deserialize, Serialize};

use crate::{ZaiResult, client::error::RealtimeErrorKind};

/// Input audio format for `session.update`.
///
/// `Wav` ⇒ 16 kHz, `Wav48` ⇒ 48 kHz (per the official protocol).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum InputAudioFormat {
    /// 16 kHz WAV.
    #[default]
    #[serde(rename = "wav")]
    Wav,
    /// 48 kHz WAV.
    #[serde(rename = "wav48")]
    Wav48,
}

/// Output audio format for `session.update`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum OutputAudioFormat {
    /// Raw PCM (server default).
    #[default]
    #[serde(rename = "pcm")]
    Pcm,
    /// MP3 frames.
    #[serde(rename = "mp3")]
    Mp3,
}

/// Standard (non-URL) base64-encode, matching the wire format used by the
/// realtime API.
pub fn encode_base64(data: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(data)
}

/// Standard base64-decode of a realtime audio payload.
pub fn decode_base64(s: &str) -> ZaiResult<Vec<u8>> {
    base64::engine::general_purpose::STANDARD
        .decode(s)
        .map_err(|e| RealtimeErrorKind::Protocol(format!("base64 decode failed: {e}")).into())
}

/// Wrap raw 16-bit little-endian mono PCM samples in a minimal WAV header and
/// return the base64-encoded file body for `input_audio_buffer.append`.
///
/// `samples` must be the raw PCM byte stream (two bytes per sample, mono).
pub fn encode_wav_pcm_base64(samples: &[u8], sample_rate: u32) -> String {
    let bytes_per_sample: u32 = 2;
    let channels: u32 = 1;
    // Saturating math: WAV sizes are 32-bit, so guard against silent `as`
    // truncation on absurdly large inputs rather than emitting a corrupt
    // header (a >4 GiB PCM buffer can't be represented in WAV anyway).
    let byte_rate = sample_rate
        .saturating_mul(channels)
        .saturating_mul(bytes_per_sample);
    let block_align = (channels * bytes_per_sample).min(u16::MAX as u32) as u16;
    let data_len = u32::try_from(samples.len()).unwrap_or(u32::MAX);
    let chunk_size = data_len.saturating_add(36);

    let mut wav = Vec::with_capacity(44 + samples.len());
    // RIFF header
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&chunk_size.to_le_bytes());
    wav.extend_from_slice(b"WAVE");
    // fmt chunk
    wav.extend_from_slice(b"fmt ");
    wav.extend_from_slice(&16u32.to_le_bytes()); // PCM fmt chunk size
    wav.extend_from_slice(&1u16.to_le_bytes()); // PCM
    wav.extend_from_slice(&(channels as u16).to_le_bytes());
    wav.extend_from_slice(&sample_rate.to_le_bytes());
    wav.extend_from_slice(&byte_rate.to_le_bytes());
    wav.extend_from_slice(&block_align.to_le_bytes());
    wav.extend_from_slice(&((bytes_per_sample * 8) as u16).to_le_bytes()); // bits per sample
    // data chunk
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_len.to_le_bytes());
    wav.extend_from_slice(samples);

    encode_base64(&wav)
}

/// Base64-encode a JPEG video frame for
/// `input_audio_buffer.append_video_frame`.
pub fn encode_jpeg_frame_base64(jpg: &[u8]) -> String {
    encode_base64(jpg)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wav_round_trip_has_valid_header() {
        // 100 samples of silence = 200 bytes of PCM.
        let pcm = vec![0u8; 200];
        let wav_b64 = encode_wav_pcm_base64(&pcm, 16000);
        let wav = decode_base64(&wav_b64).unwrap();
        assert_eq!(&wav[0..4], b"RIFF");
        assert_eq!(&wav[8..12], b"WAVE");
        assert_eq!(&wav[12..16], b"fmt ");
        assert_eq!(&wav[22..24], 1u16.to_le_bytes()); // mono
        assert_eq!(&wav[24..28], 16000u32.to_le_bytes()); // sample rate
        assert_eq!(&wav[34..36], 16u16.to_le_bytes()); // bits per sample
        assert_eq!(&wav[36..40], b"data");
        assert_eq!(&wav[40..44], (pcm.len() as u32).to_le_bytes()); // data length
        assert_eq!(&wav[44..], &pcm[..]);
    }

    #[test]
    fn base64_round_trip() {
        let data = b"hello realtime";
        assert_eq!(decode_base64(&encode_base64(data)).unwrap(), data);
    }
}
