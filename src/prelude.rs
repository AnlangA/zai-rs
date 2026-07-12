//! Prelude for the zai-rs 0.5 public surface (plan P10.1).
//!
//! `use zai_rs::prelude::*;` brings in the four core types plus the essential
//! model types needed for chat construction.
//!
//! ```text
//! use zai_rs::prelude::*;
//!
//! let client = ZaiClient::builder("key").build()?;
//! let response = client.services().chat();
//! ```

pub use crate::client::{HttpTransportConfig, ZaiClient, ZaiClientBuilder, ZaiError, ZaiResult};

// Essential model types for chat construction.
pub use crate::model::traits::{ModelName, StreamOff as NonStreaming, StreamOn as Streaming};
pub use crate::model::{ChatCompletion as ChatRequest, TextMessage as ChatMessage};

#[cfg(feature = "mcp")]
pub use crate::mcp::*;
