//! # ZAI-RS: Zhipu AI Rust SDK
//!
//! `zai-rs` is a type-safe Rust SDK providing full coverage of the Zhipu AI
//! (BigModel) API. Strongly-typed clients and models span chat completions,
//! image generation, speech recognition, text embeddings, knowledge-base
//! management, and more.
//!
//! # Capabilities
//!
//! | Capability | Description | Module |
//! |------------|-------------|--------|
//! | Chat completions | Sync / async / streaming text, vision, voice | [`model`] |
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
//! | Agent | Agent creation & management | [`agent`] |
//! | Tool execution framework | Dynamic registration, execution, caching | [`toolkits`] |
//! | Real-time | WebSocket audio/video (GLM-Realtime) | [`realtime`] |
//! | Coding Plan usage | GLM Coding Plan quota / 余量查询 | [`usage`] |
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
//! - [`agent`] — Agent API (creation, chat, history)
//! - [`toolkits`] — Tool execution framework (registration, execution, caching,
//!   RMCP bridge)
//! - [`realtime`] — Real-time audio/video communication (WebSocket,
//!   experimental)
//! - [`usage`] — Coding Plan usage / quota query (GLM Coding Plan 余量查询)
//!
//! # Quick Start
//!
//! ```text
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
//! # Streaming Requests
//!
//! ```text
//! use zai_rs::{client::ZaiClient, model::*};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let model = GLM4_5_flash {};
//!     let client = ZaiClient::from_env()?;
//!     let request = ChatCompletion::new(model, TextMessage::user("Hello"));
//!     let _response = request.send_via(&client).await?;
//!     Ok(())
//! }
//! ```
//!
//! # Configuration
//!
//! `ZaiConfig` is the central place for credentials, endpoint families, and
//! HTTP transport settings. It mirrors the API families exposed by
//! [`client::EndpointConfig`], including the dedicated Coding Plan
//! endpoint required by official Zhipu AI documentation.
//!
//! ```text
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! use zai_rs::ZaiConfig;
//!
//! let config = ZaiConfig::builder()
//!     .api_key("abc123.abcdefghijklmnopqrstuvwxyz")
//!     .paas_v4_base("https://open.bigmodel.cn/api/paas/v4")
//!     .coding_paas_v4_base("https://open.bigmodel.cn/api/coding/paas/v4")
//!     .build()?;
//!
//! assert_eq!(
//!     config.coding_paas_v4_url("chat/completions"),
//!     "https://open.bigmodel.cn/api/coding/paas/v4/chat/completions"
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
//! | `toolkits` | disabled | Tool execution with JSON-Schema argument validation |
//!
//! Enable in `Cargo.toml`:
//! ```toml
//! [dependencies]
//! zai-rs = { version = "0.5", features = ["mcp"] }
//! ```
//!
//! # Error Handling
//!
//! All API calls return `ZaiResult<T>`,
//! unified under the [`ZaiError`] enum:
//!
//! - `ApiError` — Business-level API error (with code and message)
//! - `NetworkError` — Network / timeout error
//! - `JsonError` — JSON serialization / deserialization error
//! - `RateLimitError` — Rate-limit or quota exceeded
//! - `ContentPolicyError` — API policy or unsafe-content block
//! - `AuthError` — Authentication / authorization error
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

pub mod agent;
pub mod batches;
pub mod client;
pub use client::{ZaiClient, error::*};
pub mod file;
pub mod knowledge;

/// Unified MCP capability client.
#[cfg(feature = "mcp")]
pub mod mcp;

pub mod model;
/// WebSocket realtime (GLM-Realtime) client — audio/video over a WebSocket.
/// Gated behind the `realtime` Cargo feature (off by default).
#[cfg(feature = "realtime")]
pub mod realtime;
pub mod services;
pub mod tool;
pub mod toolkits;
pub mod usage;

pub mod prelude;
