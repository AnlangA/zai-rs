//! Text-to-speech synthesis (TTS) with buffered WAV/PCM or streaming PCM.
//!
//! [`TextToAudioRequest::enable_stream`](crate::model::text_to_audio::TextToAudioRequest::enable_stream)
//! changes the request type-state, forces PCM, and exposes
//! [`TtsEncodeFormat`](crate::model::text_to_audio::TtsEncodeFormat) only on the
//! streaming builder. The resulting
//! [`TextToAudioStream`](crate::model::text_to_audio::TextToAudioStream) yields
//! decoded [`bytes::Bytes`] chunks.

/// Request builder and client for text-to-speech.
mod data;
/// Supported TTS model ids.
mod model;
/// Request body types.
mod request;

pub use data::*;
pub use model::*;
pub use request::*;

#[cfg(test)]
pub(crate) use model::TTS_MODEL_REGISTRY_SNAPSHOT;
