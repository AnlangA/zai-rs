//! Speech-to-text (ASR) — transcribe audio into text.

/// Request builder and client for audio transcription.
pub mod data;
/// Supported ASR model ids.
pub mod model;
/// Request body types.
pub mod request;
/// Response body types.
pub mod response;

pub use data::*;
pub use model::*;
pub use request::*;
pub use response::*;
