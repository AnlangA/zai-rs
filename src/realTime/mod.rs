//! # Real-time API Module
//!
//! Provides real-time audio/video communication via WebSocket for the Zhipu AI
//! API. Designed for interactive applications requiring low-latency streaming.
//!
//! > **Note:** The framework is in place; audio/video call features are still
//! > under active development.
//!
//! # Core Types
//!
//! - [`RealTimeClient`] — Entry point for real-time sessions
//! - [`RealTimeModel`] — Supported model identifiers
//! - [`RealTimeSession`] — Manages a single real-time session
//!
//! # Usage
//!
//! ```rust,ignore
//! use zai_rs::realTime::*;
//!
//! let client = RealTimeClient::new(api_key);
//! let session = client.audio_session()
//!     .model(RealTimeModel::Glm4Voice)
//!     .build()
//!     .await?;
//! ```

pub mod client;
pub mod models;
pub mod session;
pub mod types;

pub use client::*;
pub use models::*;
pub use session::*;
pub use types::*;
