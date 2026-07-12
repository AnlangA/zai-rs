//! Voice cloning — create a cloned voice from audio samples.

/// Request builder and client for voice cloning.
mod data;
/// Supported voice-clone model ids.
mod model;
/// Request body types.
mod request;
/// Response body types.
mod response;

pub use data::*;
pub use model::*;
pub use request::*;
pub use response::*;
