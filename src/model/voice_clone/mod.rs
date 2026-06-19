//! Voice cloning — create a cloned voice from audio samples.

/// Request builder and client for voice cloning.
pub mod data;
/// Supported voice-clone model ids.
pub mod model;
/// Request body types.
pub mod request;
/// Response body types.
pub mod response;

pub use data::*;
pub use model::*;
pub use request::*;
pub use response::*;
