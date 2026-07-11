//! HTTP client infrastructure: the shared [`ZaiClient`], validated endpoints,
//! transport policies, error types, and the legacy send-path free functions.

pub mod config;
pub mod endpoint;
pub mod error;
pub mod http;
pub mod secret;
pub mod services;
pub mod transport;

// Re-export the main public types.
pub use config::{
    AdditionalHeader, HttpTransportConfig, HttpTransportConfigBuilder, RetryOverride, ZaiClient,
    ZaiClientBuilder,
};
pub use endpoint::{ApiFamily, EndpointConfig, EndpointConfigBuilder};
pub use error::*;
pub use services::Services;
