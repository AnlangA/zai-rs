//! # Model Module
//!
//! Contains all data models, request/response types, and API abstractions for the Zhipu AI API.
//! This module provides type-safe representations of API entities and comprehensive support
//! for various AI capabilities.
//!
//! ## Module Organization
//!
//! The module is organized into several categories:
//!
//! ### Chat & Conversation
//! - [`chat`] - Synchronous chat completion
//! - [`async_chat`] - Asynchronous chat completion
//! - [`async_chat_get`] - Retrieving async chat results
//! - [`chat_message_types`] - Message types for different conversation modes
//! - [`chat_stream_response`] - Streaming response handling
//!
//! ### Multimodal AI
//! - [`audio_to_text`] - Speech recognition (ASR)
//! - [`audio_to_speech`] - Text-to-speech synthesis (TTS)
//! - [`gen_image`] - Image generation
//! - [`gen_video_async`] - Video generation (async)
//!
//! ### Voice & Audio
//! - [`voice_clone`] - Voice cloning capabilities
//! - [`voice_list`] - Voice management and listing
//! - [`voice_delete`] - Voice deletion
//!
//! ### Core Infrastructure
//! - [`chat_base_request`] - Base request structures
//! - [`chat_base_response`] - Base response structures
//! - [`chat_models`] - AI model definitions
//! - [`tools`] - Tool calling and function definitions
//! - [`traits`] - Core traits and abstractions
//! - [`model_validate`] - Data validation utilities
//! - [`stream_ext`] - Streaming extensions
//!
//! ## Key Features
//!
//! - **Type Safety** - Compile-time guarantees for API usage
//! - **Model Validation** - Built-in data validation
//! - **Streaming Support** - Real-time response processing
//! - **Multimodal Support** - Text, vision, voice, and audio capabilities
//! - **Tool Integration** - Function calling and external tool support
//!
//! ## Usage Example
//!
//! ```rust,ignore
//! use zai_rs::model::*;
//!
//! // Create a chat completion request
//! let model = GLM4_5_flash {};
//! let messages = TextMessage::user("Hello, how can you help me?");
//! let client = ChatCompletion::new(model, messages, api_key);
//! ```

pub mod async_chat;
pub mod async_chat_get;
pub mod audio_to_speech;
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
pub mod stream_ext;
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
pub use stream_ext::StreamChatLikeExt;

pub use chat_message_types::*;
pub use chat_models::*;
pub use gen_video_async::*;
pub use tools::*;

pub use chat_base_response::TaskStatus;
pub use chat_stream_response::ChatStreamResponse;
pub use traits::SseStreamable;
