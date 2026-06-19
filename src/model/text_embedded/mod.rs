//! Text embeddings — generate vector embeddings for text.

/// Request builder and client for embeddings.
pub mod data;
/// Request body types.
pub mod request;
/// Response body types.
pub mod response;

pub use data::*;
pub use request::*;
pub use response::*;
