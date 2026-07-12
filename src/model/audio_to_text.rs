//! Speech-to-text (ASR) — transcribe audio into text.

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
