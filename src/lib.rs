//! # ZAI-RS: Zhipu AI Rust SDK
//!
//! `zai-rs` is a type-safe Rust SDK that provides complete support for the
//! Zhipu AI (BigModel) APIs. It offers strongly typed API clients and models
//! for a wide range of AI capabilities including chat, image generation,
//! speech recognition, and text-to-speech.
//!
//! ## Capabilities
//!
//! - Chat completions (text, vision, and voice)
//! - Image generation
//! - Speech-to-text (audio transcription)
//! - Text-to-speech (audio synthesis)
//! - Tool/function calling integration
//! - File management (upload, list, content, delete)
//! - Streaming responses via Server-Sent Events (SSE)
//!
//! ## Module Structure
//!
//! - [`client`] — HTTP client and networking
//! - [`model`] — Data models, API request/response types
//! - [`mod@file`] — File management features
//! - [`batches`] — Batch processing endpoints (list batches)
//! - [`toolkits`] — Tool calling and execution framework
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use zai_rs::model::*;
//! use zai_rs::client::http::*;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let model = GLM4_5_flash {};
//!     let key = std::env::var("ZHIPU_API_KEY").unwrap();
//!     let client = ChatCompletion::new(model, TextMessage::user("Hello"), key);
//!     let _resp = client.post().await?;
//!     Ok(())
//! }
//! ```
//!
//! ## Features
//!
//! - Type safety with compile-time checks to minimize runtime errors
//! - Async support powered by Tokio
//! - Streaming support for real-time responses
//! - Tool integration for function calling and external tools
//! - Built-in validation and error handling

pub mod client;
pub mod file;
pub mod batches;
pub mod knowledge;

pub mod model;
pub mod tool;
pub mod toolkits;
