//! # HTTP Client Module
//!
//! Provides HTTP client infrastructure for communicating with the Zhipu AI API,
//! including request dispatch, response parsing, connection pooling, automatic
//! retries, and comprehensive error handling.
//!
//! # Core Components
//!
//! - [`http`] — HTTP client implementation supporting POST/GET/DELETE requests,
//!   connection reuse, exponential-backoff retries, and sensitive-data masking
//! - [`error`] — Unified error type [`ZaiError`] covering API, network,
//!   serialization, validation, and retry-exhausted errors
//! - [`wss`] — WebSocket Secure connection support (for real-time audio/video)
//!
//! # Retry Strategy
//!
//! The HTTP client ships a configurable retry mechanism:
//!
//! - **Exponential backoff** — default; delay doubles after each attempt
//! - **Fixed delay** — constant interval between retries
//! - **Random jitter** — adds randomness on top of backoff to avoid thundering
//!   herds
//! - **Configurable limits** — max retries and overall timeout are customizable
//!
//! ```rust,ignore
//! use zai_rs::client::http::HttpClientConfig;
//! use std::time::Duration;
//!
//! let config = HttpClientConfig::builder()
//!     .max_retries(5)
//!     .timeout(Duration::from_secs(120))
//!     .build();
//! ```
//!
//! # Security
//!
//! - API keys are automatically masked in logs via
//!   [`mask_sensitive_info`](error::mask_sensitive_info)
//! - Structured logging of requests/responses through `tracing`
//! - Bearer token authentication on every request

pub mod error;
pub mod http;
pub mod wss;

pub use error::*;
pub use http::*;
