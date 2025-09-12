//! # Streaming Extensions for Chat-like Endpoints
//!
//! This module provides typed streaming capabilities for chat completion APIs
//! that return Server-Sent Events (SSE) with `ChatStreamResponse` chunks.
//!
//! ## Features
//!
//! - **Callback-based API** - Simple async closure interface for processing chunks
//! - **Stream-based API** - Composable, testable, and reusable stream interface
//! - **Type-safe parsing** - Automatic deserialization of SSE data chunks
//! - **Error handling** - Comprehensive error propagation and handling
//!
//! ## Usage Patterns
//!
//! ### Callback-based Processing
//! ```rust,ignore
//! client.stream_for_each(|chunk| async move {
//!     println!("Received: {:?}", chunk);
//!     Ok(())
//! }).await?;
//! ```
//!
//! ### Stream-based Processing
//! ```rust,ignore
//! let mut stream = client.to_stream().await?;
//! while let Some(result) = stream.next().await {
//!     match result {
//!         Ok(chunk) => println!("Chunk: {:?}", chunk),
//!         Err(e) => eprintln!("Error: {}", e),
//!     }
//! }
//! ```

use crate::client::http::HttpClient;
use crate::model::chat_stream_response::ChatStreamResponse;
use crate::model::traits::SseStreamable;
use futures::{Stream, StreamExt, stream};
use log::info;
use std::pin::Pin;

/// Streaming extension trait for chat-like endpoints.
///
/// This trait provides two complementary APIs for processing streaming responses:
/// 1. **Callback-based** - Simple async closure interface
/// 2. **Stream-based** - Composable stream interface for advanced usage
///
/// Both APIs handle SSE protocol parsing, JSON deserialization, and error propagation.
pub trait StreamChatLikeExt: SseStreamable + HttpClient {
    /// Processes streaming responses using an async callback function.
    ///
    /// This method provides a simple interface for handling streaming chat responses.
    /// Each successfully parsed chunk is passed to the provided callback function.
    ///
    /// ## Arguments
    ///
    /// * `on_chunk` - Async callback function that processes each `ChatStreamResponse` chunk
    ///
    /// ## Returns
    ///
    /// Result indicating success or failure of the streaming operation
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// client.stream_for_each(|chunk| async move {
    ///     if let Some(content) = &chunk.choices[0].delta.content {
    ///         print!("{}", content);
    ///     }
    ///     Ok(())
    /// }).await?;
    /// ```
    fn stream_for_each<'a, F, Fut>(
        &'a mut self,
        mut on_chunk: F,
    ) -> impl core::future::Future<Output = anyhow::Result<()>> + 'a
    where
        F: FnMut(ChatStreamResponse) -> Fut + 'a,
        Fut: core::future::Future<Output = anyhow::Result<()>> + 'a,
    {
        async move {
            let resp = self.post().await?;
            let mut stream = resp.bytes_stream();
            let mut buf: Vec<u8> = Vec::new();

            while let Some(next) = stream.next().await {
                let bytes = match next {
                    Ok(b) => b,
                    Err(e) => return Err(anyhow::anyhow!("Stream error: {}", e)),
                };
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
                        if let Ok(chunk) = serde_json::from_slice::<ChatStreamResponse>(rest) {
                            on_chunk(chunk).await?;
                        }
                    }
                }
            }
            Ok(())
        }
    }

    /// Converts the streaming response into a composable Stream.
    ///
    /// This method returns a `Stream` that yields `ChatStreamResponse` chunks,
    /// enabling advanced stream processing operations like filtering, mapping,
    /// and combination with other streams.
    ///
    /// ## Returns
    ///
    /// A future that resolves to a `Stream` of `Result<ChatStreamResponse>` items
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// let stream = client.to_stream().await?;
    /// let collected: Vec<_> = stream
    ///     .filter_map(|result| result.ok())
    ///     .collect()
    ///     .await;
    /// ```
    fn to_stream<'a>(
        &'a mut self,
    ) -> impl core::future::Future<
        Output = anyhow::Result<
            Pin<Box<dyn Stream<Item = anyhow::Result<ChatStreamResponse>> + Send + 'static>>,
        >,
    > + 'a {
        async move {
            let resp = self.post().await?;
            let byte_stream = resp.bytes_stream();

            // State: (byte_stream, buffer)
            let s = byte_stream;

            let out = stream::unfold((s, Vec::<u8>::new()), |(mut s, mut buf)| async move {
                loop {
                    // Process all complete lines currently in buffer
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
                                return None; // end stream gracefully
                            }
                            match serde_json::from_slice::<ChatStreamResponse>(rest) {
                                Ok(item) => return Some((Ok(item), (s, buf))),
                                Err(_) => { /* skip invalid json line */ }
                            }
                        }
                    }
                    // Need more bytes
                    match s.next().await {
                        Some(Ok(bytes)) => buf.extend_from_slice(&bytes),
                        Some(Err(e)) => {
                            return Some((Err(anyhow::anyhow!("Stream error: {}", e)), (s, buf)));
                        }
                        None => return None,
                    }
                }
            })
            .boxed();

            Ok(out)
        }
    }
}
