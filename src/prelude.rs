//! Convenience imports for the most common client and chat APIs.
//!
//! `use zai_rs::prelude::*;` imports the shared client, transport
//! configuration, error types, chat request/message aliases, and model
//! type-state traits. Enabling the `mcp` feature also adds the MCP API.
//!
//! ```rust,no_run
//! use zai_rs::{model::GLM4_5_flash, prelude::*};
//!
//! # async fn demo() -> ZaiResult<()> {
//! let client = ZaiClient::builder("key").build()?;
//! let request = ChatRequest::new(GLM4_5_flash {}, ChatMessage::user("Hello"));
//! let response = request.send_via(&client).await?;
//! # Ok(())
//! # }
//! ```

pub use crate::client::{
    ApiFamily, HttpConcurrencyConfig, HttpTransportConfig, RequestOptions, RetryOverride,
    ZaiClient, ZaiClientBuilder, ZaiError, ZaiResult,
};

// Keep the prelude intentionally small; specialized request/response types stay
// in their semantic modules.
pub use crate::model::traits::{ModelName, StreamOff as NonStreaming, StreamOn as Streaming};
pub use crate::model::{ChatCompletion as ChatRequest, TextMessage as ChatMessage};

#[cfg(feature = "mcp")]
pub use crate::mcp::*;
