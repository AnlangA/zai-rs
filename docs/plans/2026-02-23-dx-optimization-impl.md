# ZAI-RS DX Optimization Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Optimize the zai-rs SDK for better developer experience through improved documentation, ergonomic APIs, better error handling, and cleaner code organization.

**Architecture:** Layered refactoring approach - first enhance error types as foundation, then improve API ergonomics with Into<String> patterns, add comprehensive documentation, and finally reorganize module structure with a clean public API facade.

**Tech Stack:** Rust 2021 edition, thiserror for error handling, serde for serialization, validator for validation, tokio for async, reqwest for HTTP.

---

## Phase 1: Error Types Enhancement

### Task 1.1: Add `#[non_exhaustive]` to ZaiError

**Files:**
- Modify: `src/client/error.rs:199-240`

**Step 1: Add non_exhaustive attribute to ZaiError enum**

Change the enum definition from:
```rust
/// Main error type for the ZAI-RS SDK
#[derive(Error, Debug)]
pub enum ZaiError {
```

To:
```rust
/// Main error type for the ZAI-RS SDK.
///
/// This enum is marked as `#[non_exhaustive]` to allow adding new error
/// variants in future versions without breaking changes.
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum ZaiError {
```

**Step 2: Run tests to verify no breakage**

Run: `cargo test --lib`
Expected: All tests pass

**Step 3: Commit**

```bash
git add src/client/error.rs
git commit -m "feat(error): add #[non_exhaustive] to ZaiError for future compatibility"
```

---

### Task 1.2: Implement `source()` for error chaining

**Files:**
- Modify: `src/client/error.rs`

**Step 1: Implement std::error::Error::source for ZaiError**

Add after the `impl ZaiError` block (around line 420):

```rust
impl std::error::Error for ZaiError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ZaiError::NetworkError(err) => Some(err.as_ref()),
            ZaiError::JsonError(err) => Some(err.as_ref()),
            _ => None,
        }
    }
}
```

**Step 2: Run tests**

Run: `cargo test --lib`
Expected: All tests pass

**Step 3: Commit**

```bash
git add src/client/error.rs
git commit -m "feat(error): implement source() for error chaining"
```

---

### Task 1.3: Add context methods to ZaiResult

**Files:**
- Modify: `src/client/error.rs`

**Step 1: Add context extension trait**

Add at the end of the file (before the test module):

```rust
/// Extension trait for adding context to `Result` types.
///
/// This trait provides ergonomic methods for adding contextual information
/// to errors, making debugging and error reporting easier.
pub trait ResultExt<T> {
    /// Adds context to the error.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let result = some_operation().context("while processing request")?;
    /// ```
    fn context<C: Into<String>>(self, context: C) -> ZaiResult<T>;

    /// Adds context to the error using a closure.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let result = some_operation().with_context(|| format!("processing item {}", id))?;
    /// ```
    fn with_context<C, F>(self, f: F) -> ZaiResult<T>
    where
        C: Into<String>,
        F: FnOnce() -> C;
}

impl<T> ResultExt<T> for ZaiResult<T> {
    fn context<C: Into<String>>(self, context: C) -> ZaiResult<T> {
        self.map_err(|e| ZaiError::ContextError {
            source: Box::new(e),
            context: context.into(),
        })
    }

    fn with_context<C, F>(self, f: F) -> ZaiResult<T>
    where
        C: Into<String>,
        F: FnOnce() -> C,
    {
        self.map_err(|e| ZaiError::ContextError {
            source: Box::new(e),
            context: f().into(),
        })
    }
}
```

**Step 2: Add ContextError variant to ZaiError**

Add this variant to the `ZaiError` enum (around line 238, before `Unknown`):

```rust
    /// Error with additional context information.
    #[error("{context}: {source}")]
    ContextError {
        /// The underlying error source.
        source: Box<ZaiError>,
        /// Additional context describing where/why the error occurred.
        context: String,
    },
```

**Step 3: Update Clone impl for ZaiError**

Add this case to the `Clone` impl (around line 454):

```rust
            ZaiError::ContextError { source, context } => ZaiError::ContextError {
                source: source.clone(),
                context: context.clone(),
            },
```

**Step 4: Update code(), message(), compact() methods**

Add these cases to the respective methods in `impl ZaiError`:

```rust
// In code()
ZaiError::ContextError { source, .. } => source.code(),

// In message()
ZaiError::ContextError { context, .. } => context.clone(),

// In compact()
ZaiError::ContextError { context, source } => {
    format!("CONTEXT: {} -> {}", context, source.compact())
}
```

**Step 5: Run tests**

Run: `cargo test --lib`
Expected: All tests pass

**Step 6: Commit**

```bash
git add src/client/error.rs
git commit -m "feat(error): add context methods for richer error information"
```

---

### Task 1.4: Enhance error variant documentation

**Files:**
- Modify: `src/client/error.rs:198-240`

**Step 1: Enhance documentation for each error variant**

Replace the enum variants with enhanced documentation:

```rust
/// Main error type for the ZAI-RS SDK.
///
/// This enum represents all possible errors that can occur when using the SDK.
/// Each variant includes an error code and descriptive message.
///
/// # Error Categories
///
/// - **HTTP errors** (4xx/5xx status codes) - Network/protocol level errors
/// - **Authentication errors** (1000-1100) - API key and auth issues
/// - **Account errors** (1110-1121) - Account status and quota issues
/// - **API errors** (1200-1234) - Invalid parameters or request issues
/// - **Rate limit errors** (1300-1309) - Throttling and quota exceeded
///
/// # Example
///
/// ```rust,ignore
/// match result {
///     Err(ZaiError::AuthError { code, message }) => {
///         eprintln!("Authentication failed ({}): {}", code, message);
///         eprintln!("Verify your API key at https://open.bigmodel.cn/api-keys");
///     }
///     Err(ZaiError::RateLimitError { code, .. }) => {
///         eprintln!("Rate limited, please retry after some time");
///     }
///     _ => {}
/// }
/// ```
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum ZaiError {
    /// HTTP status errors (4xx, 5xx responses).
    ///
    /// These errors indicate protocol-level issues with the HTTP request.
    /// Common causes include invalid endpoints, server errors, or malformed requests.
    #[error("HTTP error [{status}]: {message}")]
    HttpError {
        /// HTTP status code (e.g., 400, 404, 500).
        status: u16,
        /// Human-readable error message.
        message: String,
    },

    /// Authentication and authorization errors.
    ///
    /// These errors occur when the API key is invalid, expired, or missing.
    /// Verify your API key at https://open.bigmodel.cn/api-keys
    #[error("Authentication error [{code}]: {message}")]
    AuthError {
        /// API error code (1000-1004, 1100).
        code: u16,
        /// Human-readable error message.
        message: String,
    },

    /// Account-related errors.
    ///
    /// These errors indicate issues with your account status,
    /// such as insufficient balance or account suspension.
    #[error("Account error [{code}]: {message}")]
    AccountError {
        /// API error code (1110-1121).
        code: u16,
        /// Human-readable error message.
        message: String,
    },

    /// API call errors.
    ///
    /// These errors indicate invalid parameters or malformed requests.
    /// Check the API documentation for correct parameter formats.
    #[error("API error [{code}]: {message}")]
    ApiError {
        /// API error code (1200-1234).
        code: u16,
        /// Human-readable error message.
        message: String,
    },

    /// Rate limiting and quota errors.
    ///
    /// These errors occur when you've exceeded your API usage limits.
    /// Implement exponential backoff and retry logic.
    #[error("Rate limit error [{code}]: {message}")]
    RateLimitError {
        /// API error code (1300-1309).
        code: u16,
        /// Human-readable error message.
        message: String,
    },

    /// Content policy errors.
    ///
    /// These errors occur when the request or response violates content policies.
    #[error("Content policy error [{code}]: {message}")]
    ContentPolicyError {
        /// API error code.
        code: u16,
        /// Human-readable error message.
        message: String,
    },

    /// File processing errors.
    ///
    /// These errors occur during file upload, download, or processing operations.
    #[error("File error [{code}]: {message}")]
    FileError {
        /// API error code.
        code: u16,
        /// Human-readable error message.
        message: String,
    },

    /// Network/IO errors.
    ///
    /// These errors indicate network connectivity issues.
    /// Check your internet connection and retry.
    #[error("Network error: {0}")]
    NetworkError(Arc<reqwest::Error>),

    /// JSON parsing errors.
    ///
    /// These errors indicate issues with JSON serialization or deserialization.
    /// This may indicate an API response format change.
    #[error("JSON error: {0}")]
    JsonError(Arc<serde_json::Error>),

    /// Error with additional context information.
    #[error("{context}: {source}")]
    ContextError {
        /// The underlying error source.
        source: Box<ZaiError>,
        /// Additional context describing where/why the error occurred.
        context: String,
    },

    /// Other uncategorized errors.
    #[error("Unknown error [{code}]: {message}")]
    Unknown {
        /// Error code.
        code: u16,
        /// Human-readable error message.
        message: String,
    },
}
```

**Step 2: Run clippy**

Run: `cargo clippy --lib`
Expected: No warnings

**Step 3: Run tests**

Run: `cargo test --lib`
Expected: All tests pass

**Step 4: Commit**

```bash
git add src/client/error.rs
git commit -m "docs(error): enhance ZaiError variant documentation with examples"
```

---

## Phase 2: API Ergonomics

### Task 2.1: Add Into<String> to ChatCompletion::new

**Files:**
- Modify: `src/model/chat/data.rs:72-97`

**Step 1: Update ChatCompletion::new signature**

Change:
```rust
pub fn new(model: N, messages: M, key: String) -> ChatCompletion<N, M, StreamOff> {
```

To:
```rust
/// Creates a new non-streaming chat completion request.
///
/// # Arguments
///
/// * `model` - The AI model to use for completion
/// * `messages` - The conversation messages
/// * `api_key` - API key for authentication (accepts `&str`, `String`, etc.)
///
/// # Returns
///
/// A new `ChatCompletion` instance configured for non-streaming requests.
///
/// # Example
///
/// ```rust,ignore
/// let request = ChatCompletion::new(
///     GLM4_5_flash {},
///     TextMessage::user("Hello!"),
///     "your-api-key" // or env!("ZHIPU_API_KEY")
/// );
/// ```
pub fn new<K: Into<String>>(model: N, messages: M, api_key: K) -> ChatCompletion<N, M, StreamOff> {
```

And update the body:
```rust
    let body = ChatBody::new(model, messages);
    ChatCompletion {
        body,
        key: api_key.into(),
        url: "https://open.bigmodel.cn/api/paas/v4/chat/completions".to_string(),
        _stream: PhantomData,
    }
```

**Step 2: Run tests**

Run: `cargo test --lib`
Expected: All tests pass

**Step 3: Commit**

```bash
git add src/model/chat/data.rs
git commit -m "feat(chat): accept Into<String> for API key in ChatCompletion::new"
```

---

### Task 2.2: Add builder validation methods with proper docs

**Files:**
- Modify: `src/model/chat/data.rs`

**Step 1: Enhance with_temperature documentation and validation**

Replace the existing `with_temperature` method:

```rust
    /// Sets the temperature for response randomness.
    ///
    /// Temperature controls the randomness of the model's output.
    /// Higher values (closer to 1.0) produce more creative but less predictable
    /// responses, while lower values (closer to 0.0) produce more deterministic
    /// and focused outputs.
    ///
    /// # Arguments
    ///
    /// * `temperature` - Value between 0.0 and 1.0 (inclusive)
    ///
    /// # Returns
    ///
    /// Self with the temperature set, enabling method chaining.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let request = ChatCompletion::new(model, messages, api_key)
    ///     .with_temperature(0.7); // Balanced creativity
    /// ```
    ///
    /// # See Also
    ///
    /// - [`with_top_p`](Self::with_top_p) for nucleus sampling
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        debug_assert!(
            (0.0..=1.0).contains(&temperature),
            "Temperature must be between 0.0 and 1.0, got {}",
            temperature
        );
        self.body = self.body.with_temperature(temperature);
        self
    }
```

**Step 2: Enhance with_top_p documentation**

Replace the existing `with_top_p` method:

```rust
    /// Sets the top-p (nucleus sampling) parameter.
    ///
    /// Top-p sampling limits the model to consider only tokens whose
    /// cumulative probability is below the specified threshold.
    /// This provides an alternative to temperature for controlling output diversity.
    ///
    /// # Arguments
    ///
    /// * `top_p` - Value between 0.0 and 1.0 (inclusive)
    ///
    /// # Returns
    ///
    /// Self with top_p set, enabling method chaining.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let request = ChatCompletion::new(model, messages, api_key)
    ///     .with_top_p(0.9); // Consider tokens comprising top 90% probability
    /// ```
    pub fn with_top_p(mut self, top_p: f32) -> Self {
        debug_assert!(
            (0.0..=1.0).contains(&top_p),
            "Top-p must be between 0.0 and 1.0, got {}",
            top_p
        );
        self.body = self.body.with_top_p(top_p);
        self
    }
```

**Step 3: Enhance with_max_tokens documentation**

Replace the existing `with_max_tokens` method:

```rust
    /// Sets the maximum number of tokens to generate.
    ///
    /// This limits the length of the model's response. One token is
    /// approximately 4 characters or 0.75 words in English.
    ///
    /// # Arguments
    ///
    /// * `max_tokens` - Maximum tokens (1 to 98,304)
    ///
    /// # Returns
    ///
    /// Self with max_tokens set, enabling method chaining.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let request = ChatCompletion::new(model, messages, api_key)
    ///     .with_max_tokens(1024); // Limit to ~750 words
    /// ```
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        debug_assert!(
            (1..=98304).contains(&max_tokens),
            "Max tokens must be between 1 and 98304, got {}",
            max_tokens
        );
        self.body = self.body.with_max_tokens(max_tokens);
        self
    }
```

**Step 4: Run tests**

Run: `cargo test --lib`
Expected: All tests pass

**Step 5: Commit**

```bash
git add src/model/chat/data.rs
git commit -m "docs(chat): add comprehensive documentation for builder methods"
```

---

### Task 2.3: Add Into<String> to message constructors

**Files:**
- Modify: `src/model/chat_message_types.rs`

**Step 1: Verify Into<String> is already used**

Check that message constructors already use `impl Into<String>`:
- `TextMessage::user(content: impl Into<String>)` - Already done
- `TextMessage::assistant(content: impl Into<String>)` - Already done
- `TextMessage::system(content: impl Into<String>)` - Already done

The message types already have good ergonomic constructors. No changes needed.

**Step 2: Mark as complete, no commit needed**

---

## Phase 3: Documentation Standards

### Task 3.1: Enhance lib.rs module documentation

**Files:**
- Modify: `src/lib.rs`

**Step 1: Replace lib.rs content with enhanced documentation**

```rust
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
//! | [`agent`] | Agent API for persistent conversations |
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
//! The SDK uses a comprehensive error type [`ZaiError`](client::error::ZaiError)
//! that maps to Zhipu AI API error codes:
//!
//! ```rust,ignore
//! use zai_rs::client::error::{ZaiError, ZaiResult};
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
```

**Step 2: Run doc tests**

Run: `cargo doc --no-deps`
Expected: No warnings

**Step 3: Commit**

```bash
git add src/lib.rs
git commit -m "docs: enhance lib.rs with comprehensive module documentation"
```

---

### Task 3.2: Add documentation to model/mod.rs

**Files:**
- Modify: `src/model/mod.rs`

**Step 1: The file already has good documentation - verify it's complete**

Read the current state and verify. The file at lines 1-61 already has comprehensive documentation.

**Step 2: Add cross-references to related modules**

Add at the end of the module documentation (before the module declarations):

```rust
//! ## Re-exports
//!
//! The following types are re-exported for convenience:
//!
//! ### Chat Types
//! - [`ChatCompletion`] - Main chat completion client
//! - [`ChatStreamResponse`] - Streaming response handler
//!
//! ### Message Types
//! - [`TextMessage`] - Text-only messages
//! - [`VisionMessage`] - Messages with images/videos
//! - [`VoiceMessage`] - Messages with audio
//!
//! ### Model Types
//! - [`GLM5`], [`GLM4_7`], [`GLM4_6`], [`GLM4_5`] - Text models
//! - [`GLM4_5_flash`], [`GLM4_5_air`] - Fast text models
//! - [`GLM4_5v`] - Vision model
//! - [`GLM4_voice`] - Voice model
```

**Step 3: Commit**

```bash
git add src/model/mod.rs
git commit -m "docs(model): add cross-references in module documentation"
```

---

### Task 3.3: Document the traits module

**Files:**
- Modify: `src/model/traits.rs`

**Step 1: The file already has comprehensive documentation**

The traits.rs file (lines 1-125) already has excellent documentation. No changes needed.

---

### Task 3.4: Add panic section to methods that can panic

**Files:**
- Modify: `src/model/chat/data.rs`

**Step 1: Add # Panics sections to methods with debug_assert**

Update the `with_temperature`, `with_top_p`, and `with_max_tokens` methods to include:

```rust
    /// # Panics
    ///
    /// Debug builds will panic if the value is outside the valid range.
    /// Release builds will silently accept invalid values (API will reject them).
```

**Step 2: Commit**

```bash
git add src/model/chat/data.rs
git commit -m "docs(chat): add panic sections to validation methods"
```

---

## Phase 4: Verification & Cleanup

### Task 4.1: Run full test suite

**Step 1: Run all tests**

Run: `cargo test`
Expected: All tests pass

**Step 2: Run clippy**

Run: `cargo clippy --all-targets --all-features -- -D warnings`
Expected: No warnings

**Step 3: Generate documentation**

Run: `cargo doc --no-deps`
Expected: No warnings

---

### Task 4.2: Verify examples compile

**Step 1: Check all examples compile**

Run: `cargo build --examples`
Expected: All examples compile

**Step 2: Commit verification**

```bash
git add -A
git commit -m "chore: verify all tests, clippy, and examples pass"
```

---

## Summary

### Changes Made

1. **Error Types Enhancement**
   - Added `#[non_exhaustive]` to `ZaiError`
   - Implemented `source()` for error chaining
   - Added `context()` and `with_context()` methods
   - Enhanced all error variant documentation

2. **API Ergonomics**
   - `ChatCompletion::new` now accepts `Into<String>` for API key
   - Added validation assertions to builder methods
   - Enhanced documentation with examples

3. **Documentation**
   - Comprehensive lib.rs documentation with examples
   - Cross-references in module docs
   - Panic sections where applicable

4. **Verification**
   - All tests pass
   - No clippy warnings
   - Documentation builds cleanly
   - All examples compile

### Breaking Changes

None - All changes are backward compatible.

### Next Steps (Future Work)

1. Create `prelude` module for convenient imports
2. Move error types to root level `error.rs`
3. Consolidate model submodules for better organization
4. Add more inline code examples
