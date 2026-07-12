//! Retrieve the result of an asynchronous chat, image, or video task.

/// Request builder for fetching an asynchronous task result.
mod data;
/// Shared acknowledgement and polling response types.
mod response;

pub use data::*;
pub use response::*;
