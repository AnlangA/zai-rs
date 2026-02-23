//! # ZAI-RS: Zhipu AI Rust SDK
//!
//! `zai-rs` is a type-safe, ergonomic Rust SDK for the Zhipu AI (BigModel) API.
//! It provides strongly-typed API clients for AI capabilities including chat,
//! image generation, speech recognition, and more.
//!
//! ## Features
//!
//! - **Type-Safe** - Compile-time guarantees prevent invalid API calls
//! - **Async** - Built on Tokio for efficient async I/O
//! - **Streaming** - SSE streaming for real-time responses
//! - **Multimodal** - Text, vision, voice, and audio support
//! - **Tool Calling** - Function calling and web search integration
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use zai_rs::model::{ChatCompletion, GLM4_5_flash, TextMessage};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let response = ChatCompletion::new(
//!         GLM4_5_flash {},
//!         TextMessage::user("Hello, how can you help me?"),
//!         std::env::var("ZHIPU_API_KEY")?
//!     )
//!     .with_temperature(0.7)
//!     .send()
//!     .await?;
//!
//!     if let Some(content) = &response.choices[0].message.content {
//!         println!("{}", content);
//!     }
//!     Ok(())
//! }
//! ```
//!
//! ## Module Organization
//!
//! | Module | Description |
//! |--------|-------------|
//! | [`model`] | AI models, messages, and request/response types |
//! | [`client`] | HTTP client and networking utilities |
//! | [`toolkits`] | Tool calling and execution framework |
//! | [`batches`] | Batch processing for multiple requests |
//! | [`file`] | File upload and management |
//! | [`knowledge`] | Knowledge base operations |
//! | [`tool`] | External tool APIs (web search, file parsing) |
//! | [`io`] | Unified file I/O operations |
//!
//! ## Supported Models
//!
//! | Model | Text | Vision | Voice | Thinking | Tool Stream |
//! |-------|------|--------|-------|----------|-------------|
//! | GLM-5 | ✓ | ✗ | ✗ | ✓ | ✓ |
//! | GLM-4.7 | ✓ | ✗ | ✗ | ✓ | ✓ |
//! | GLM-4.6 | ✓ | ✗ | ✗ | ✓ | ✓ |
//! | GLM-4.5 | ✓ | ✗ | ✗ | ✓ | ✗ |
//! | GLM-4.5-Flash | ✓ | ✗ | ✗ | ✓ | ✗ |
//! | GLM-4.5V | ✓ | ✓ | ✗ | ✗ | ✗ |
//! | GLM-4-Voice | ✓ | ✗ | ✓ | ✗ | ✗ |
//!
//! ## Error Handling
//!
//! The SDK uses a comprehensive error type [`ZaiError`] that maps to
//! Zhipu AI API error codes:
//!
//! [`ZaiError`]: client::error::ZaiError
//!
//! ```rust,ignore
//! use zai_rs::client::error::{ZaiError, ZaiResult, ResultExt};
//!
//! async fn handle_error(result: ZaiResult<Response>) {
//!     match result {
//!         Ok(response) => { /* handle success */ },
//!         Err(ZaiError::AuthError { code, message }) => {
//!             eprintln!("Auth failed ({}): {}", code, message);
//!         },
//!         Err(ZaiError::RateLimitError { .. }) => {
//!             eprintln!("Rate limited, please retry");
//!         },
//!         Err(e) => eprintln!("Error: {}", e),
//!     }
//! }
//! ```
//!
//! ## Streaming
//!
//! Enable streaming for real-time response processing:
//!
//! ```rust,ignore
//! use zai_rs::model::{ChatCompletion, GLM4_5_flash, TextMessage, StreamChatLikeExt};
//!
//! let mut stream = ChatCompletion::new(
//!     GLM4_5_flash {},
//!     TextMessage::user("Tell me a story"),
//!     api_key
//! )
//! .enable_stream();
//!
//! stream.stream_sse_for_each(|data| {
//!     println!("Received chunk: {:?}", String::from_utf8_lossy(data));
//! }).await?;
//! ```
//!
//! ## API Documentation
//!
//! For full Zhipu AI API documentation, visit:
//! <https://open.bigmodel.cn/dev/api>

pub mod batches;
pub mod client;
pub use client::error::*;
pub mod file;
pub mod io;
pub mod knowledge;

pub mod model;
pub mod tool;
pub mod toolkits;
