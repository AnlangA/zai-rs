//! # Model Module
//!
//! Contains all data models, request/response types, and API abstractions for
//! the Zhipu AI API. This module provides type-safe representations of API
//! entities with comprehensive support for various AI capabilities.
//!
//! # Module Organization
//!
//! ## Chat & Conversation
//!
//! - [`chat`] — Synchronous chat completion
//! - [`async_chat`] — Asynchronous (queued) chat completion
//! - [`async_chat_get`] — Retrieve async chat results
//! - [`chat_message_types`] — Message types for text, vision, and voice modes
//! - [`chat_base_request`] — Shared request body (`ChatBody`)
//! - [`chat_base_response`] — Shared response structures
//! - [`chat_stream_response`] — Streaming response deserialization
//!
//! ## Multimodal AI
//!
//! - [`gen_image`] — Image generation
//! - [`gen_video_async`] — Async video generation
//! - [`audio_to_text`] — Speech recognition (ASR)
//! - [`text_to_audio`] — Text-to-speech synthesis (TTS)
//! - [`ocr`] — Optical character recognition
//!
//! ## Text Analysis
//!
//! - [`text_embedded`] — Text embeddings
//! - [`text_rerank`] — Re-ranking
//! - [`text_tokenizer`] — Tokenization
//! - [`moderation`] — Content moderation / safety analysis
//!
//! ## Voice Management
//!
//! - [`voice_clone`] — Voice cloning
//! - [`voice_list`] — Voice listing
//! - [`voice_delete`] — Voice deletion
//!
//! ## Infrastructure
//!
//! - [`chat_models`] — Model type definitions and capability markers
//! - [`tools`] — Tool/function definitions, `ThinkingType`, web-search tools
//! - [`traits`] — Core traits (`Chat`, `AsyncChat`, `Bounded`, `SseStreamable`,
//!   etc.)
//! - [`model_validate`] — Request validation helpers
//! - [`sse_parser`] — SSE protocol parser
//! - [`stream_ext`] — Stream extension traits
//!
//! # Key Design Patterns
//!
//! - **Marker traits** — [`Chat`], [`AsyncChat`], [`ThinkEnable`] etc. encode
//!   model capabilities at compile time
//! - **Type-state pattern** — [`StreamOn`](traits::StreamOn) /
//!   [`StreamOff`](traits::StreamOff) enforce streaming vs. non-streaming at
//!   the type level
//! - **Bounded pairing** — the [`Bounded`](traits::Bounded) trait ties model
//!   types to compatible message types, preventing invalid combinations at
//!   compile time
//!
//! # Usage
//!
//! ```rust,ignore
//! use zai_rs::model::*;
//!
//! let model = GLM4_5_flash {};
//! let messages = TextMessage::user("Hello, how can you help me?");
//! let client = ChatCompletion::new(model, messages, api_key);
//! ```

pub mod async_chat;
pub mod async_chat_get;
pub mod audio_to_text;
pub mod chat;
pub mod chat_base_request;
pub mod chat_base_response;
pub mod chat_message_types;
pub mod chat_models;
pub mod chat_stream_response;
pub mod gen_image;
pub mod gen_video_async;
pub mod model_validate;
pub mod moderation;
pub mod ocr;
pub mod sse_parser;
pub mod stream_ext;
pub mod text_embedded;
pub mod text_rerank;
pub mod text_to_audio;
pub mod text_tokenizer;
pub mod tools;
pub mod traits;
pub mod voice_clone;
pub mod voice_delete;
pub mod voice_list;

// Avoid wildcard re-exports to prevent name collisions (e.g., `data`)

// Selective type re-exports for convenience
pub use async_chat::data::AsyncChatCompletion;
pub use async_chat_get::data::AsyncChatGetRequest;
pub use chat::data::ChatCompletion;
pub use chat_base_response::TaskStatus;
pub use chat_message_types::*;
pub use chat_models::*;
pub use chat_stream_response::ChatStreamResponse;
pub use gen_video_async::*;
pub use moderation::data::Moderation;
pub use stream_ext::StreamChatLikeExt;
pub use tools::*;
pub use traits::SseStreamable;
