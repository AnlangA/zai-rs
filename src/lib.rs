//! # ZAI-RS: Zhipu AI Rust SDK
//!
//! `zai-rs` is a type-safe Rust SDK for the Zhipu AI (BigModel) API.
//! Strongly-typed clients and models span chat completions,
//! image generation, speech recognition, text embeddings, knowledge-base
//! management, and more.
//!
//! # Capabilities
//!
//! | Capability | Description | Module |
//! |------------|-------------|--------|
//! | Chat completions | Sync / async text, vision, and voice | [`model`] |
//! | Image generation | Text-to-image | [`model::gen_image`] |
//! | Video generation | Async text-to-video | [`model::gen_video_async`] |
//! | Text-to-speech | Audio synthesis | [`model::text_to_audio`] |
//! | Speech-to-text | Audio transcription | [`model::audio_to_text`] |
//! | Voice cloning | Voice clone, list, delete | [`model::voice_clone`] |
//! | Text embeddings | Embeddings, reranking, tokenization | [`model::text_embedded`] |
//! | Content moderation | Safety analysis | [`model::moderation`] |
//! | OCR | Handwriting recognition | [`model::ocr`] |
//! | File management | Upload, list, content, delete | [`mod@file`] |
//! | Batch processing | Create, list, retrieve, cancel | [`batches`] |
//! | Knowledge base | CRUD, document upload, retrieval | [`knowledge`] |
//! | Tool calling | Function calling, web search, file parsing | [`tool`] |
//! | MCP | Unified search, reader, repository, and vision capabilities | [`mcp`] |
//! | Agent | Agent v1 invocation, async polling, conversation continuation | [`agent`] |
//! | Tool execution framework | Dynamic registration, execution, caching | [`toolkits`] |
//! | Real-time | WebSocket audio/video (GLM-Realtime) | [`realtime`] |
//! | Coding Plan usage | GLM Coding Plan quota and remaining usage | [`usage`] |
//!
//! # Module Structure
//!
//! - [`client`] — HTTP client, connection pool, retry strategy, error types
//! - [`model`] — Data models, request/response types, model definitions, SSE
//!   parsing
//! - [`mod@file`] — File management (upload, list, content, delete)
//! - [`batches`] — Batch processing (create, list, retrieve, cancel)
//! - [`knowledge`] — Knowledge-base management (CRUD, document upload,
//!   retrieval)
//! - [`tool`] — Tool implementations (web search, file parsing)
//! - [`mcp`] — Unified MCP capabilities with automatic backend and transport
//!   selection (feature `mcp`)
//! - [`agent`] — Agent v1 invocation, async-result polling, and conversation
//!   continuation
//! - [`toolkits`] — Tool execution framework (registration, execution, caching,
//!   RMCP bridge)
//! - [`realtime`] — Real-time audio/video communication (WebSocket,
//!   experimental)
//! - [`usage`] — Coding Plan quota and remaining-usage query
//!
//! # Quick Start
//!
//! ```rust,no_run
//! use zai_rs::{client::ZaiClient, model::*};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let model = GLM4_5_flash {};
//!     let client = ZaiClient::from_env()?;
//!     let request = ChatCompletion::new(model, TextMessage::user("Hello"));
//!     let _resp = request.send_via(&client).await?;
//!     Ok(())
//! }
//! ```
//!
//! # Configuration
//!
//! [`ZaiClient`] owns credentials, validated endpoint families, connection
//! pooling, timeouts, and retry policy. Clone the client to share the same
//! transport safely across requests.
//!
//! ```rust,no_run
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! use zai_rs::client::{ApiFamily, ZaiClient};
//!
//! let client = ZaiClient::builder("abc123.abcdefghijklmnopqrstuvwxyz")
//!     .endpoint(
//!         ApiFamily::CodingPaasV4,
//!         "https://open.bigmodel.cn/api/coding/paas/v4",
//!     )
//!     .build()?;
//!
//! assert_eq!(
//!     client.endpoints().base(ApiFamily::CodingPaasV4).as_str(),
//!     "https://open.bigmodel.cn/api/coding/paas/v4"
//! );
//! # Ok(())
//! # }
//! ```
//!
//! # Feature Flags
//!
//! | Feature | Default | Description |
//! |---------|---------|-------------|
//! | (default) | enabled | Core API functionality |
//! | `realtime` | disabled | Real-time audio/video over WebSocket (GLM-Realtime) |
//! | `mcp` | disabled | Unified high-level MCP capability client |
//! | `rmcp-kits` | disabled | Enable RMCP protocol bridge for MCP tool calling |
//! | `toolkits` | disabled | JSON-Schema validation for the tool-execution framework |
//!
//! Enable in `Cargo.toml`:
//! ```toml
//! [dependencies]
//! zai-rs = { version = "6.0.1", features = ["mcp"] }
//! ```
//!
//! # Error Handling
//!
//! All API calls return `ZaiResult<T>`,
//! unified under the [`ZaiError`] enum:
//!
//! Error variants distinguish HTTP, authentication, account, API, rate-limit,
//! content-policy, file, network, JSON, realtime, and unknown failures. Use
//! [`ZaiError::category`](client::ZaiError::category) when recovery logic only
//! needs a coarse classification.
//!
//! # Design Principles
//!
//! - **Compile-time type safety** — trait bounds and type-state patterns ensure
//!   model/message compatibility at compile time
//! - **Zero-cost abstractions** — marker traits and type-state patterns impose
//!   no runtime overhead
//! - **Consistent API style** — request builders carry typed payloads and all
//!   network operations are dispatched with `send_via(&ZaiClient)`

// On docs.rs (which builds with `--cfg docsrs`, see `[package.metadata.docs.rs]`
// in Cargo.toml), enable the nightly `doc_cfg` feature so feature-gated items
// are badged in the rendered documentation. The `cfg_attr` is inert on stable
// local builds, where `docsrs` is never set.
#![cfg_attr(docsrs, feature(doc_cfg))]
// Public API documentation is part of the compatibility surface. Keep missing
// docs visible in normal development and fatal under the workspace CI gate.
#![warn(missing_docs)]

pub mod agent;
pub mod batches;
pub mod client;
pub use client::{ZaiClient, error::*};
pub mod file;
pub mod knowledge;

/// Unified MCP capability client.
#[cfg(feature = "mcp")]
#[cfg_attr(docsrs, doc(cfg(feature = "mcp")))]
pub mod mcp;

pub mod model;
/// WebSocket realtime (GLM-Realtime) client — audio/video over a WebSocket.
/// Gated behind the `realtime` Cargo feature (off by default).
#[cfg(feature = "realtime")]
#[cfg_attr(docsrs, doc(cfg(feature = "realtime")))]
pub mod realtime;
mod serde_helpers;
/// Typed service facades for application, assistant, image, and document tools.
pub mod services;
pub mod tool;
pub mod toolkits;
pub mod usage;

pub mod prelude;
