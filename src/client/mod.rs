//! HTTP client infrastructure: the shared [`ZaiClient`], validated endpoints,
//! transport policies and error types.

pub mod config;
pub mod endpoint;
pub mod error;
pub(crate) mod routes;
pub mod secret;
pub mod transport;

// Re-export the main public types.
pub use config::{
    AdditionalHeader, HttpTransportConfig, HttpTransportConfigBuilder, RetryOverride, ZaiClient,
    ZaiClientBuilder,
};
pub use endpoint::{ApiFamily, EndpointConfig, EndpointConfigBuilder};
pub use error::*;
