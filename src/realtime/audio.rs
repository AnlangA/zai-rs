//! Audio format + base64 helpers for the realtime API.
//!
//! The GLM-Realtime protocol exchanges audio as base64-encoded payloads:
//!
//! - **Input** (`input_audio_buffer.append`): base64-encoded WAV or raw PCM.
//!   Raw PCM declares its sample rate in `input_audio_format` (`"pcm16"` for
//!   16 kHz or `"pcm24"` for 24 kHz).
//! - **Output** (`response.audio.delta`): base64-encoded raw 24 kHz, mono,
//!   16-bit PCM. The session decodes each delta before handing it to callers.

use base64::Engine;
use serde::{Deserialize, Serialize};
use std::io::Write;

use crate::{ZaiResult, client::error::RealtimeErrorKind};

/// Input audio format for `session.update`.
///
/// The current GLM-Realtime protocol accepts WAV or raw PCM at 16/24 kHz.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum InputAudioFormat {
    /// A WAV container. [`RealtimeSession::send_audio`](super::RealtimeSession::send_audio)
    /// creates a mono, 16-bit, 16 kHz WAV when this format is selected.
    #[default]
    #[serde(rename = "wav")]
    Wav,
    /// Raw 16-bit little-endian mono PCM sampled at 16 kHz.
    #[serde(rename = "pcm16")]
    Pcm16,
    /// Raw 16-bit little-endian mono PCM sampled at 24 kHz.
    #[serde(rename = "pcm24")]
    Pcm24,
}

/// Output audio format for `session.update`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum OutputAudioFormat {
    /// Raw 24 kHz, mono, 16-bit PCM (the only current output format).
    #[default]
    #[serde(rename = "pcm")]
    Pcm,
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
pub fn encode_wav_pcm_base64(samples: &[u8], sample_rate: u32) -> ZaiResult<String> {
    if !samples.len().is_multiple_of(2) {
        return Err(RealtimeErrorKind::Protocol(
            "16-bit PCM input must contain an even number of bytes".into(),
        )
        .into());
    }
    if sample_rate == 0 {
        return Err(RealtimeErrorKind::Protocol("WAV sample rate must be positive".into()).into());
    }

    let bytes_per_sample: u32 = 2;
    let channels: u32 = 1;
    let byte_rate = sample_rate
        .checked_mul(channels * bytes_per_sample)
        .ok_or_else(|| RealtimeErrorKind::Protocol("WAV byte rate overflow".into()))?;
    let block_align = u16::try_from(channels * bytes_per_sample)
        .map_err(|_| RealtimeErrorKind::Protocol("WAV block alignment overflow".into()))?;
    let data_len = u32::try_from(samples.len())
        .map_err(|_| RealtimeErrorKind::Protocol("PCM input is too large for WAV".into()))?;
    let chunk_size = data_len
        .checked_add(36)
        .ok_or_else(|| RealtimeErrorKind::Protocol("PCM input is too large for WAV".into()))?;
    let total_len = samples
        .len()
        .checked_add(44)
        .ok_or_else(|| RealtimeErrorKind::Protocol("PCM input is too large for WAV".into()))?;
    let encoded_len = base64::encoded_len(total_len, true)
        .ok_or_else(|| RealtimeErrorKind::Protocol("PCM input is too large for base64".into()))?;

    // Build only the fixed-size header locally, then stream the header and PCM
    // directly into the base64 destination. This avoids retaining a second
    // full-size WAV Vec beside the encoded payload.
    let mut header = [0u8; 44];
    // RIFF header
    header[0..4].copy_from_slice(b"RIFF");
    header[4..8].copy_from_slice(&chunk_size.to_le_bytes());
    header[8..12].copy_from_slice(b"WAVE");
    // fmt chunk
    header[12..16].copy_from_slice(b"fmt ");
    header[16..20].copy_from_slice(&16u32.to_le_bytes()); // PCM fmt chunk size
    header[20..22].copy_from_slice(&1u16.to_le_bytes()); // PCM
    header[22..24].copy_from_slice(&(channels as u16).to_le_bytes());
    header[24..28].copy_from_slice(&sample_rate.to_le_bytes());
    header[28..32].copy_from_slice(&byte_rate.to_le_bytes());
    header[32..34].copy_from_slice(&block_align.to_le_bytes());
    header[34..36].copy_from_slice(&((bytes_per_sample * 8) as u16).to_le_bytes());
    // data chunk
    header[36..40].copy_from_slice(b"data");
    header[40..44].copy_from_slice(&data_len.to_le_bytes());

    let mut encoded = Vec::with_capacity(encoded_len);
    {
        let mut encoder = base64::write::EncoderWriter::new(
            &mut encoded,
            &base64::engine::general_purpose::STANDARD,
        );
        encoder
            .write_all(&header)
            .and_then(|()| encoder.write_all(samples))
            .and_then(|()| encoder.finish().map(|_| ()))
            .map_err(|error| {
                RealtimeErrorKind::Protocol(format!("base64 encode failed: {error}"))
            })?;
    }
    String::from_utf8(encoded).map_err(|error| {
        RealtimeErrorKind::Protocol(format!("base64 encoder produced invalid UTF-8: {error}"))
            .into()
    })
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
        let wav_b64 = encode_wav_pcm_base64(&pcm, 16000).unwrap();
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

    #[test]
    fn wav_encoder_rejects_invalid_pcm_metadata() {
        assert!(encode_wav_pcm_base64(&[0], 16_000).is_err());
        assert!(encode_wav_pcm_base64(&[0, 0], 0).is_err());
    }

    #[test]
    fn current_formats_use_official_wire_values() {
        assert_eq!(
            serde_json::to_string(&InputAudioFormat::Wav).unwrap(),
            r#""wav""#
        );
        assert_eq!(
            serde_json::to_string(&InputAudioFormat::Pcm16).unwrap(),
            r#""pcm16""#
        );
        assert_eq!(
            serde_json::to_string(&InputAudioFormat::Pcm24).unwrap(),
            r#""pcm24""#
        );
        assert_eq!(
            serde_json::to_string(&OutputAudioFormat::Pcm).unwrap(),
            r#""pcm""#
        );
        assert!(serde_json::from_str::<InputAudioFormat>(r#""wav48""#).is_err());
        assert!(serde_json::from_str::<OutputAudioFormat>(r#""mp3""#).is_err());
    }
}
