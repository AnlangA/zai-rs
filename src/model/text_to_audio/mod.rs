//! Text-to-speech synthesis (TTS) — generate audio from text.

/// Request builder and client for text-to-speech.
pub mod data;
/// Supported TTS model ids.
pub mod model;
/// Request body types.
pub mod request;

pub use data::*;
pub use model::*;
pub use request::*;
