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
//! | Agent | Agent creation & management | [`agent`] |
//! | Tool execution framework | Dynamic registration, execution, caching | [`toolkits`] |
//! | Real-time | WebSocket audio/video (framework ready) | [`realTime`] |
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
//! - [`agent`] — Agent API (creation, chat, history)
//! - [`toolkits`] — Tool execution framework (registration, execution, caching,
//!   RMCP bridge)
//! - [`realTime`] — Real-time audio/video communication (WebSocket)
//!
//! # Quick Start
//!
//! ```rust,no_run
//! use zai_rs::{client::http::*, model::*};
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
//! # Streaming Responses
//!
//! ```rust,no_run
//! use zai_rs::{client::http::*, model::*};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let model = GLM4_5_flash {};
//!     let key = std::env::var("ZHIPU_API_KEY").unwrap();
//!     let mut client =
//!         ChatCompletion::new(model, TextMessage::user("Hello"), key).enable_stream();
//!     client
//!         .stream_sse_for_each(|data| {
//!             print!("{}", String::from_utf8_lossy(data));
//!         })
//!         .await?;
//!     Ok(())
//! }
//! ```
//!
//! # Feature Flags
//!
//! | Feature | Default | Description |
//! |---------|---------|-------------|
//! | (default) | enabled | Core API functionality |
//! | `rmcp-kits` | disabled | Enable RMCP protocol bridge for MCP tool calling |
//! | `web-example` | disabled | Enable axum/tower dependencies for web examples |
//!
//! Enable in `Cargo.toml`:
//! ```toml
//! [dependencies]
//! zai-rs = { version = "0.1", features = ["rmcp-kits"] }
//! ```
//!
//! # Error Handling
//!
//! All API calls return [`ZaiResult`](client::error::ZaiResult)`<T>`,
//! unified under the [`ZaiError`](client::error::ZaiError) enum:
//!
//! - `ApiError` — Business-level API error (with code and message)
//! - `NetworkError` — Network / timeout error
//! - `JsonError` — JSON serialization / deserialization error
//! - `RateLimitError` — Rate-limit or quota exceeded
//! - `AuthError` — Authentication / authorization error
//!
//! # Design Principles
//!
//! - **Compile-time type safety** — trait bounds and type-state patterns ensure
//!   model/message compatibility at compile time
//! - **Zero-cost abstractions** — marker traits and type-state patterns impose
//!   no runtime overhead
//! - **Consistent API style** — all API clients follow a uniform builder
//!   pattern and implement the `HttpClient` trait

pub mod agent;
pub mod batches;
pub mod client;
pub use client::error::*;
pub mod file;
pub mod knowledge;

pub mod model;
#[allow(non_snake_case)]
pub mod realTime;
pub mod tool;
pub mod toolkits;
