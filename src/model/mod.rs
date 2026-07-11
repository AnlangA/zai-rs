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
//! - **Marker traits** — [`Chat`](traits::Chat),
//!   [`AsyncChat`](traits::AsyncChat), [`ThinkEnable`](traits::ThinkEnable)
//!   etc. encode model capabilities at compile time
//! - **Type-state pattern** — [`StreamOn`](traits::StreamOn) /
//!   [`StreamOff`](traits::StreamOff) enforce streaming vs. non-streaming at
//!   the type level
//! - **Bounded pairing** — the [`Bounded`](traits::Bounded) trait ties model
//!   types to compatible message types, preventing invalid combinations at
//!   compile time
//!
//! # Usage
//!
//! ```text
//! use zai_rs::model::*;
//!
//! let model = GLM4_5_flash {};
//! let messages = TextMessage::user("Hello, how can you help me?");
//! let client = ChatCompletion::new(model, messages, api_key);
//! ```

/// Asynchronous (queued) chat completion — submit a chat task and poll later.
pub mod async_chat;
/// Retrieve the result of an asynchronous chat task.
pub mod async_chat_get;
/// Speech-to-text (ASR) — transcribe audio into text.
pub mod audio_to_text;
/// Synchronous chat completion (text / vision / voice).
pub mod chat;
/// Shared chat request body ([`chat_base_request::ChatBody`]).
pub mod chat_base_request;
/// Shared chat response structures ([`chat_base_response`]).
pub mod chat_base_response;
/// Message types for text, vision, and voice chat modes.
pub mod chat_message_types;
/// Model type definitions and capability marker traits.
pub mod chat_models;
/// Streaming chat response deserialization.
pub mod chat_stream_response;
/// Text-to-image generation.
pub mod gen_image;
/// Asynchronous text-to-video generation.
pub mod gen_video_async;
/// Request validation helpers.
pub mod model_validate;
/// Content moderation / safety analysis.
pub mod moderation;
/// Optical character recognition (OCR).
pub mod ocr;
/// Server-Sent Events (SSE) protocol parser.
pub mod sse_parser;
/// Stream extension traits for chat streaming.
pub mod stream_ext;
/// Text embeddings.
pub mod text_embedded;
/// Text re-ranking.
pub mod text_rerank;
/// Text-to-speech synthesis (TTS).
pub mod text_to_audio;
/// Tokenization / token counting.
pub mod text_tokenizer;
/// Tool/function definitions, `ThinkingType`, web-search tools.
pub mod tools;
/// Core traits (`Chat`, `AsyncChat`, `Bounded`, `SseStreamable`, …).
pub mod traits;
/// Voice cloning.
pub mod voice_clone;
/// Voice deletion.
pub mod voice_delete;
/// Voice listing.
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
