//! Optical character recognition (OCR) — recognize text/handwriting in images.

/// Request builder and client for OCR.
mod data;
/// Request body types.
mod request;
/// Response body types.
mod response;

pub use data::*;
pub use request::*;
pub use response::*;
