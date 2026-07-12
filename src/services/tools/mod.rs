//! Tools service operations (plan P06).
//!
//! These are NEW operations under `ApiFamily::PaasV4`, distinct from the
//! existing [`crate::tool`] module (web search, file parser).

mod request;
mod response;

pub use request::*;
pub use response::*;
