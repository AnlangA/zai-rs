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
    ) -> impl core::future::Future<Output = crate::ZaiResult<()>> + 'a
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
                    Err(e) => {
                        return Err(crate::client::error::ZaiError::NetworkError(e.to_string()));
                    }
                }
            }
            Ok(())
        }
    }
}
