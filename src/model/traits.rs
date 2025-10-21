//! # Core Traits for AI Model Abstractions
//!
//! This module defines the fundamental traits that enable type-safe interactions
//! with different AI models and capabilities in the Zhipu AI ecosystem.
//!
//! ## Trait Categories
//!
//! ### Model Identification
//! - [`ModelName`] - Converts model types to string identifiers
//!
//! ### Capability Traits
//! - [`Chat`] - Synchronous chat completion capability
//! - [`AsyncChat`] - Asynchronous chat completion capability
//! - [`ThinkEnable`] - Thinking/reasoning capability support
//! - [`VideoGen`] - Video generation capability
//! - [`ImageGen`] - Image generation capability
//! - [`AudioToText`] - Speech recognition capability
//! - [`TextToSpeech`] - Text-to-speech capability
//! - [`VoiceClone`] - Voice cloning capability
//!
//! ### Type Safety
//! - [`Bounded`] - Compile-time model-message compatibility verification
//! - [`StreamState`] - Type-state pattern for streaming capability control
//!
//! ## Type-State Pattern
//!
//! The `StreamState` trait and its implementations (`StreamOn`, `StreamOff`) provide
//! compile-time guarantees about streaming capabilities, preventing runtime errors
//! and enabling better API design.

/// Trait for AI models that can be identified by name.
///
/// This trait enables conversion of model types to their string identifiers
/// used in API requests. All AI model types must implement this trait.
pub trait ModelName: Into<String> {}

/// Marker trait for compile-time model-message compatibility checking.
///
/// This trait is used in conjunction with the type system to ensure that
/// specific model types are only used with compatible message types,
/// preventing invalid API calls at compile time.
pub trait Bounded {}

/// Indicates that a model supports synchronous chat completion.
///
/// Models implementing this trait can be used with the chat completion API
/// API for real-time conversational interactions.
pub trait Chat {}

/// Indicates that a model supports asynchronous chat completion.
///
/// Models implementing this trait can be used with the async chat completion API
/// API for queued, background processing of conversational requests.
pub trait AsyncChat {}

/// Indicates that a model supports thinking/reasoning capabilities.
///
/// Models implementing this trait can utilize advanced reasoning modes
/// that show step-by-step thinking processes for complex problem solving.
pub trait ThinkEnable {}

/// Indicates that a model supports streaming tool calls (tool_stream parameter).
/// Only models implementing this marker can enable tool_stream in requests.
pub trait ToolStreamEnable {}

/// Indicates that a model supports video generation.
///
/// Models implementing this trait can be used to generate videos from
/// text descriptions or other inputs.
pub trait VideoGen {}

/// Indicates that a model supports image generation.
///
/// Models implementing this trait can be used to generate images from
/// text descriptions or other inputs.
pub trait ImageGen {}

/// Indicates that a model supports speech recognition.
///
/// Models implementing this trait can convert audio input to text,
/// supporting various audio formats and languages.
pub trait AudioToText {}

/// Indicates that a model supports text-to-speech synthesis.
///
/// Models implementing this trait can convert text input to audio output,
/// supporting various voices and audio formats.
pub trait TextToSpeech {}

/// Indicates that a model supports voice cloning.
///
/// Models implementing this trait can create synthetic voices that
/// mimic specific speakers based on audio samples.
pub trait VoiceClone {}

/// Type-state trait for compile-time streaming capability control.
///
/// This trait enables the type system to enforce whether a request
/// supports streaming (`StreamOn`) or non-streaming (`StreamOff`) responses,
/// preventing invalid API usage patterns.
pub trait StreamState {}

/// Type-state indicating that streaming is enabled.
///
/// Types parameterized with this marker support Server-Sent Events (SSE)
/// streaming for real-time response processing.
pub struct StreamOn;

/// Type-state indicating that streaming is disabled.
///
/// Types parameterized with this marker receive complete responses
/// rather than streaming chunks.
pub struct StreamOff;

impl StreamState for StreamOn {}
impl StreamState for StreamOff {}

use crate::client::http::HttpClient;
use futures::StreamExt;
use log::info;

/// Trait for types that support Server-Sent Events (SSE) streaming.
///
/// This trait provides streaming capabilities for API responses that support
/// real-time data transmission. The default implementation handles SSE protocol
/// parsing, logging, and callback invocation.
///
/// ## Streaming Protocol
///
/// The implementation expects SSE-formatted responses with `data: ` prefixed lines.
/// Each data line is parsed and passed to the callback function. The stream
/// terminates when a `[DONE]` marker is encountered.
///
/// ## Usage
///
/// ```rust,ignore
/// let mut client = ChatCompletion::new(model, messages, api_key).enable_stream();
/// client.stream_sse_for_each(|data| {
///     println!("Received: {}", String::from_utf8_lossy(data));
/// }).await?;
/// ```
pub trait SseStreamable: HttpClient {
    fn stream_sse_for_each<'a, F>(
        &'a mut self,
        mut on_data: F,
    ) -> impl core::future::Future<Output = anyhow::Result<()>> + 'a
    where
        F: FnMut(&[u8]) + 'a,
    {
        async move {
            let resp = self.post().await?;
            let mut stream = resp.bytes_stream();
            let mut buf: Vec<u8> = Vec::new();

            while let Some(next) = stream.next().await {
                match next {
                    Ok(bytes) => {
                        buf.extend_from_slice(&bytes);
                        while let Some(pos) = buf.iter().position(|&b| b == b'\n') {
                            let line_vec: Vec<u8> = buf.drain(..=pos).collect();
                            let mut line = &line_vec[..];
                            if line.ends_with(b"\n") {
                                line = &line[..line.len() - 1];
                            }
                            if line.ends_with(b"\r") {
                                line = &line[..line.len() - 1];
                            }
                            if line.is_empty() {
                                continue;
                            }
                            const PREFIX: &[u8] = b"data: ";
                            if line.starts_with(PREFIX) {
                                let rest = &line[PREFIX.len()..];
                                info!("SSE data: {}", String::from_utf8_lossy(rest));
                                if rest == b"[DONE]" {
                                    return Ok(());
                                }
                                on_data(rest);
                            }
                        }
                    }
                    Err(e) => return Err(anyhow::anyhow!("Stream error: {}", e)),
                }
            }
            Ok(())
        }
    }
}

/// Macro for defining AI model types with standard implementations.
///
/// This macro generates a model type with the following implementations:
/// - `Debug` and `Clone` traits
/// - `Into<String>` for API identifier conversion
/// - `Serialize` for JSON serialization
/// - `ModelName` trait marker
///
/// ## Usage Examples
///
/// ```rust,ignore
/// // Basic model definition
/// define_model_type!(GLM4_5, "glm-4.5");
///
/// // Model with attributes
/// define_model_type!(
///     #[allow(non_camel_case_types)]
///     GLM4_5_flash,
///     "glm-4.5-flash"
/// );
/// ```
#[macro_export]
macro_rules! define_model_type {
    ($(#[$meta:meta])* $name:ident, $s:expr) => {
        #[derive(Debug, Clone)]
        $(#[$meta])*
        pub struct $name {}

        impl ::core::convert::Into<String> for $name {
            fn into(self) -> String { $s.to_string() }
        }

        impl ::serde::Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where S: ::serde::Serializer {
                let model_name: String = self.clone().into();
                serializer.serialize_str(&model_name)
            }
        }

        impl $crate::model::traits::ModelName for $name {}
    };
}

/// Macro for binding message types to AI models.
///
/// This macro creates compile-time associations between model types and
/// message types, ensuring type safety in chat completion requests.
///
/// ## Usage Examples
///
/// ```rust,ignore
/// // Single message type binding
/// impl_message_binding!(GLM4_5, TextMessage);
///
/// // Multiple message type bindings
/// impl_message_binding!(GLM4_5, TextMessage, VisionMessage);
/// ```
#[macro_export]
macro_rules! impl_message_binding {
    // Single message type
    ($name:ident, $message_type:ty) => {
        impl $crate::model::traits::Bounded for ($name, $message_type) {}
    };
    // Multiple message types
    ($name:ident, $message_type:ty, $($message_types:ty),+) => {
        impl $crate::model::traits::Bounded for ($name, $message_type) {}
        $(
            impl $crate::model::traits::Bounded for ($name, $message_types) {}
        )+
    };
}

/// Macro for implementing multiple capability traits on model types.
///
/// This macro provides a convenient way to mark models with multiple
/// capabilities in a single declaration.
///
/// ## Usage Examples
///
/// ```rust,ignore
/// // Single model, multiple traits
/// impl_model_markers!(GLM4_5_flash: AsyncChat, Chat);
///
/// // Multiple models, same traits
/// impl_model_markers!([GLM4_5, GLM4_5_air]: Chat);
/// ```
#[macro_export]
macro_rules! impl_model_markers {
    // Single model, multiple markers
    ($model:ident : $($marker:path),+ $(,)?) => {
        $( impl $marker for $model {} )+
    };
    // Multiple models, multiple markers
    ([$($model:ident),+ ] : $($marker:path),+ $(,)?) => {
        $( $( impl $marker for $model {} )+ )+
    };
}
