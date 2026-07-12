//! Text-to-speech synthesis (TTS) — generate audio from text.

/// Request builder and client for text-to-speech.
mod data;
/// Supported TTS model ids.
mod model;
/// Request body types.
mod request;

pub use data::*;
pub use model::*;
pub use request::*;
