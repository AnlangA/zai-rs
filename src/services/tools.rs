//! Document-processing tool service operations.
//!
//! These operations use `ApiFamily::PaasV4` and are distinct from the
//! [`crate::tool`] module's web-search and file-parser APIs.

mod request;
mod response;

pub use request::*;
pub use response::*;
