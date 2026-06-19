//! Optical character recognition (OCR) — recognize text/handwriting in images.

/// Request builder and client for OCR.
pub mod data;
/// Supported OCR model ids.
pub mod model;
/// Request body types.
pub mod request;
/// Response body types.
pub mod response;

pub use data::*;
pub use request::*;
pub use response::*;
