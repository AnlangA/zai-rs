# ZAI-RS Developer Experience Optimization Design

**Date:** 2026-02-23
**Approach:** Layered Refactoring
**Priority:** Developer Experience (DX) Focus

## Executive Summary

This document outlines a comprehensive optimization plan for the zai-rs SDK, focusing on improving developer experience through better documentation, more ergonomic APIs, improved error handling, and cleaner code organization.

## Design Goals

1. **Professional Documentation** - Comprehensive rustdoc with examples, error sections, and proper formatting
2. **Ergonomic APIs** - Idiomatic Rust patterns (Into<String>, AsRef, builder pattern, type-state)
3. **Better Error Messages** - Rich context, actionable guidance, proper error chaining
4. **Clean Structure** - Better module organization, clear public API facade

## Layer 1: Foundation - Error Types Enhancement

### Current State
- Error types in `client/error.rs` are well-structured
- Good coverage of API error codes
- Has sensitive data masking utilities

### Improvements
1. Add `#[non_exhaustive]` to `ZaiError` enum for future compatibility
2. Implement `std::error::Error::source()` for error chaining
3. Add context methods: `.context()`, `.with_context()`
4. Improve error messages with actionable guidance
5. Add error codes documentation reference links

### Example Enhancement
```rust
/// Authentication error - API key validation failed.
///
/// This error occurs when the API key is invalid, expired, or missing.
///
/// # Common Causes
/// - API key format is incorrect (should be `id.secret`)
/// - API key has been revoked
/// - Account associated with the key is suspended
///
/// # Resolution
/// Verify your API key at https://open.bigmodel.cn/api-keys
#[error("Authentication failed [{code}]: {message}. Verify API key at https://open.bigmodel.cn/api-keys")]
AuthError { code: u16, message: String },
```

## Layer 2: API Ergonomics

### Type Signatures

**Current Pattern:**
```rust
pub fn new(model: M, messages: MSG, key: String) -> Self
```

**Improved Pattern:**
```rust
pub fn new<M, MSG, K>(model: M, messages: MSG, api_key: K) -> Self
where
    M: ModelName + Chat,
    MSG: Into<Message>,
    K: Into<String>,
```

### Builder Pattern Enhancements

Add fluent builder methods with validation:
```rust
/// Sets the temperature for response randomness.
///
/// # Arguments
/// * `temp` - Temperature value between 0.0 and 1.0
///
/// # Panics
/// Panics if temperature is outside valid range.
///
/// # Example
/// ```rust,ignore
/// let client = ChatCompletion::new(model, messages, key)
///     .with_temperature(0.7)
///     .with_max_tokens(1024);
/// ```
pub fn with_temperature(mut self, temp: f32) -> Self {
    assert!(temp >= 0.0 && temp <= 1.0, "Temperature must be between 0.0 and 1.0");
    self.temperature = Some(temp);
    self
}
```

### Message Type Improvements

Make message construction more ergonomic:
```rust
impl TextMessage {
    /// Create a user message from any string-like value.
    pub fn user<C: Into<String>>(content: C) -> Self { ... }

    /// Create an assistant message from any string-like value.
    pub fn assistant<C: Into<String>>(content: C) -> Self { ... }

    /// Create a system message from any string-like value.
    pub fn system<C: Into<String>>(content: C) -> Self { ... }
}
```

## Layer 3: Documentation Standards

### Rustdoc Template for Public APIs

Every public item should have:
1. **Summary** - One-line description
2. **Detailed Description** - Extended explanation
3. **Examples** - Runnable code examples
4. **Errors** - What errors can occur
5. **Panics** - When panics can happen
6. **Safety** - For unsafe code (if any)

### Module-Level Documentation

Each module should have:
```rust
//! # Module Name
//!
//! Brief description of the module's purpose.
//!
//! ## Overview
//!
//! Detailed explanation of the module's contents and how they work together.
//!
//! ## Key Types
//!
//! - [`TypeName`] - Description of key type
//!
//! ## Example
//!
//! ```rust,ignore
//! // Example usage of the module
//! ```
```

### Example Documentation Template

```rust
/// Performs chat completion with the configured model and messages.
///
/// This method sends a request to the Zhipu AI API and returns the
/// generated response. The request includes all configured parameters
/// such as temperature, max tokens, and tool definitions.
///
/// # Errors
///
/// Returns [`ZaiError::AuthError`] if the API key is invalid.
/// Returns [`ZaiError::RateLimitError`] if rate limits are exceeded.
/// Returns [`ZaiError::NetworkError`] if network issues occur.
///
/// # Example
///
/// ```rust,no_run
/// use zai_rs::{ChatCompletion, GLM4_5_flash, TextMessage};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let client = ChatCompletion::new(
///     GLM4_5_flash {},
///     TextMessage::user("Hello!"),
///     "your-api-key"
/// );
/// let response = client.post().await?;
/// println!("{}", response.choices[0].message.content);
/// # Ok(())
/// # }
/// ```
///
/// # See Also
///
/// - [`enable_stream`](Self::enable_stream) for streaming responses
/// - [`with_tools`](Self::with_tools) for function calling
pub async fn post(&self) -> ZaiResult<ChatResponse> { ... }
```

## Layer 4: Code Structure Improvements

### Module Reorganization

**New Structure:**
```
src/
├── lib.rs              # Clean public API facade
├── error.rs            # Move error types here (from client/error.rs)
├── prelude.rs          # Common imports (NEW)
├── client/
│   ├── mod.rs          # Re-exports + internal modules
│   ├── http.rs         # (pub(crate))
│   └── websocket.rs    # (pub(crate))
├── model/
│   ├── mod.rs          # Re-exports model types
│   ├── chat.rs         # Chat types (consolidated)
│   ├── message.rs      # Message types (consolidated from chat_message_types.rs)
│   ├── tool.rs         # Tool types (renamed from tools.rs)
│   ├── image.rs        # Image generation (renamed from gen_image)
│   ├── audio.rs        # Audio types (consolidated)
│   ├── video.rs        # Video generation (renamed from gen_video_async)
│   ├── embedding.rs    # Text embedding (renamed from text_embedded)
│   ├── rerank.rs       # Reranking (renamed from text_rerank)
│   ├── moderation.rs   # Content moderation
│   └── traits.rs       # Core traits
├── api/                # High-level API (NEW)
│   ├── mod.rs
│   ├── chat.rs         # ChatCompletion fluent builder
│   ├── image.rs        # Image generation API
│   └── audio.rs        # Audio API
├── toolkits/           # Tool calling framework
├── batches/            # Batch processing
├── file/               # File management
├── knowledge/          # Knowledge base
├── agent/              # Agent API
├── tool/               # Tool API
└── io/                 # Unified I/O
```

### Public API Facade (lib.rs)

```rust
//! # ZAI-RS: Zhipu AI Rust SDK
//!
//! A type-safe, ergonomic Rust SDK for the Zhipu AI (BigModel) API.
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use zai_rs::prelude::*;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), zai_rs::Error> {
//!     let response = ChatCompletion::new(
//!         GLM4_5_flash::new(),
//!         TextMessage::user("Hello, world!"),
//!         env!("ZHIPU_API_KEY")
//!     )
//!     .with_temperature(0.7)
//!     .post()
//!     .await?;
//!
//!     println!("{}", response.content());
//!     Ok(())
//! }
//! ```
//!
//! ## Features
//!
//! - **Type-Safe** - Compile-time guarantees prevent invalid API calls
//! - **Async** - Built on Tokio for efficient async I/O
//! - **Streaming** - SSE streaming for real-time responses
//! - **Multimodal** - Text, vision, voice, and audio support

// Re-export error types
pub use error::{Error, Result};

// Re-export core types
pub use model::{
    // Models
    GLM4_5, GLM4_5_flash, GLM4_5_air, GLM4_5_flashx,
    GLM4_6, GLM4_6_flash, GLM4_6_air, GLM4_6_flashx,
    GLM5, GLM5_flash, GLM5_air,

    // Messages
    TextMessage, VisionMessage, VoiceMessage,

    // Chat
    ChatCompletion, ChatResponse,

    // Capabilities
    Chat, AsyncChat, ThinkEnable, ToolStreamEnable,
};

// Re-export API types
pub use api::{
    chat::ChatCompletionBuilder,
    image::ImageGenerator,
    audio::{SpeechRecognizer, SpeechSynthesizer},
};

// Modules
pub mod prelude;
pub mod error;
pub mod model;
pub mod api;
pub mod toolkits;
pub mod batches;
pub mod file;
pub mod knowledge;
pub mod agent;
pub mod tool;
pub mod io;
```

### Prelude Module

```rust
//! Common imports for zai-rs.
//!
//! This module re-exports the most commonly used types and traits.
//! Import everything with `use zai_rs::prelude::*;`.

pub use crate::error::{Error, Result};
pub use crate::model::{
    // Model types
    GLM4_5, GLM4_5_flash, GLM4_5_air, GLM4_5_flashx,
    GLM4_6, GLM4_6_flash, GLM4_6_air, GLM4_6_flashx,
    GLM5, GLM5_flash, GLM5_air,

    // Message types
    TextMessage, VisionMessage, VoiceMessage,

    // Chat types
    ChatCompletion, ChatResponse, ChatStreamResponse,

    // Core traits
    Chat, AsyncChat, ModelName,
};
```

## Implementation Phases

### Phase 1: Error Types (Foundation)
1. Add `#[non_exhaustive]` to `ZaiError`
2. Implement `source()` for error chaining
3. Enhance error documentation
4. Add context methods

### Phase 2: Type Ergonomics
1. Add `Into<String>` bounds to string parameters
2. Implement builder pattern methods
3. Add validation with clear error messages
4. Improve message type constructors

### Phase 3: Documentation
1. Add module-level docs to all modules
2. Document all public types and functions
3. Add code examples with `no_run` where appropriate
4. Add error and panic sections

### Phase 4: Structure Reorganization
1. Create `error.rs` at root level
2. Create `prelude.rs` module
3. Rename/reorganize model submodules
4. Update `lib.rs` facade
5. Update all examples to work with new API

## Verification Checklist

After each phase:
- [ ] `cargo test` passes
- [ ] `cargo clippy` passes with no warnings
- [ ] `cargo doc` generates without warnings
- [ ] All 32 examples compile and run

## Breaking Changes

This optimization will introduce some breaking changes:

1. **Import paths** - Some types may move to different modules
2. **Error type name** - `ZaiError` → `Error`, `ZaiResult` → `Result`
3. **Module names** - Some modules renamed for clarity
4. **Constructor signatures** - Some may accept `Into<String>` instead of `String`

These changes are acceptable per the aggressive restructuring scope approved by the user.

## Success Criteria

1. All public APIs have comprehensive documentation
2. Error messages provide actionable guidance
3. API follows Rust idioms (Into, AsRef, builders)
4. All examples continue to work
5. No clippy warnings
6. All tests pass
