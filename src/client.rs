//! HTTP client infrastructure: the shared [`ZaiClient`], validated endpoints,
//! transport policies and error types.

pub mod config;
pub mod endpoint;
pub mod error;
pub(crate) mod operation;
pub(crate) mod query;
pub(crate) mod routes;
pub(crate) mod secret;
pub(crate) mod transport;
pub(crate) mod validation;

// Re-export the main public types.
pub use config::{
    AdditionalHeader, HttpConcurrencyConfig, HttpTransportConfig, HttpTransportConfigBuilder,
    RequestOptions, RetryOverride, ZaiClient, ZaiClientBuilder,
};
pub use endpoint::{ApiFamily, EndpointConfig, EndpointConfigBuilder};
pub use error::*;
