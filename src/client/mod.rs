//! # HTTP Client Module
//!
//! Provides HTTP communication functionality with the Zhipu AI API, including:
//! - HTTP request sending and processing
//! - Error handling and response parsing
//! - WebSocket connection support (WSS)
//!
//! ## Main Components
//!
//! - [`http`] - HTTP client implementation supporting POST and GET requests
//! - [`wss`] - WebSocket secure connection support
//!
//! ## Usage Example
//!
//! ```rust,ignore
//! use zai_rs::client::http::HttpClient;
//! // Use HttpClient trait for API calls
//! ```

pub mod http;
pub use http::*;

pub mod error;
pub use error::*;

pub mod wss;
pub use wss::*;
