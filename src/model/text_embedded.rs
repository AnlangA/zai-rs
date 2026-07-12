//! Text embeddings — generate vector embeddings for text.

/// Request builder and client for embeddings.
mod data;
/// Request body types.
mod request;
/// Response body types.
mod response;

pub use data::*;
pub use request::*;
pub use response::*;
