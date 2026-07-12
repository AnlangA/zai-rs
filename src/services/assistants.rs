//! Assistant invocation and discovery APIs.
//!
//! Request values carry endpoint-specific input while [`crate::client::ZaiClient`]
//! supplies credentials, endpoint configuration, and transport.

mod request;
mod response;

pub use request::*;
pub use response::*;
