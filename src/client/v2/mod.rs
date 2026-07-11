//! New 0.5 client architecture (plan P02).
//!
//! This module introduces the shared [`ZaiClient`] / [`ClientInner`] core,
//! the validated [`EndpointConfig`] (URL-based, scheme-checked), the
//! [`HttpTransportConfig`] policy container, the [`ApiFamily`] enum and the
//! service facades. It lives alongside the legacy 0.4 per-request types during
//! the P02–P05 migration window; P05 migrates every endpoint onto `RequestSpec`
//! and the legacy request/client paths are removed.
//!
//! # Public surface (this module)
//!
//! - [`ZaiClient`], [`ZaiClientBuilder`] — the single shared client.
//! - [`ApiFamily`] — the fixed endpoint families.
//! - [`EndpointConfig`] — validated `url::Url`-based endpoints.
//! - [`HttpTransportConfig`] — transport policy (timeouts, retry, limits,
//!   allow-listed extra headers).
//! - [`RetryOverride`] — per-request retry-safety escape hatch.
//!
//! Service facades ([`crate::client::v2::services`]) borrow a `&ZaiClient` and
//! are zero-cost to obtain.

pub mod config;
pub mod endpoint;
pub mod legacy_adapter;
pub mod services;

pub use config::{
    AdditionalHeader, HttpTransportConfig, HttpTransportConfigBuilder, RetryOverride, ZaiClient,
    ZaiClientBuilder,
};
pub use endpoint::{ApiFamily, EndpointConfig, EndpointConfigBuilder};
pub use services::Services;
