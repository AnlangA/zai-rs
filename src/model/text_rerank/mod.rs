//! Text re-ranking — re-order documents by relevance to a query.

/// Request builder and client for reranking.
pub mod data;
/// Request body types.
pub mod request;
/// Response body types.
pub mod response;

pub use data::*;
pub use request::*;
pub use response::*;
