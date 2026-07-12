//! Speech-to-text (ASR) for local or standard-base64 WAV/MP3 input.
//!
//! Requests are non-streaming by default. Calling
//! [`AudioToTextRequest::enable_stream`](crate::model::audio_to_text::AudioToTextRequest::enable_stream)
//! changes the type-state and exposes a
//! [`SpeechToTextStream`](crate::model::audio_to_text::SpeechToTextStream) of
//! [`SpeechToTextEvent`](crate::model::audio_to_text::SpeechToTextEvent) values.

/// Request builder and client for audio transcription.
mod data;
/// Supported ASR model ids.
mod model;
/// Request body types.
mod request;
/// Response body types.
mod response;

pub use data::*;
pub use model::*;
pub use request::*;
pub use response::*;
