//! # Streaming Extensions for Chat-like Endpoints
//!
//! This module provides typed streaming capabilities for chat completion APIs
//! that return Server-Sent Events (SSE) with `ChatStreamResponse` chunks.
//!
//! ## Features
//!
//! - **Callback-based API** - Simple async closure interface for processing
//!   chunks
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

use std::{collections::VecDeque, pin::Pin};

use futures::{Stream, StreamExt, stream};
use tracing::info;

use crate::{
    client::http::HttpClient,
    model::{chat_stream_response::ChatStreamResponse, traits::SseStreamable},
};

/// Streaming extension trait for chat-like endpoints.
///
/// This trait provides two complementary APIs for processing streaming
/// responses:
/// 1. **Callback-based** - Simple async closure interface
/// 2. **Stream-based** - Composable stream interface for advanced usage
///
/// Both APIs handle SSE protocol parsing, JSON deserialization, and error
/// propagation.
pub trait StreamChatLikeExt: SseStreamable + HttpClient {
    /// Processes streaming responses using an async callback function.
    ///
    /// This method provides a simple interface for handling streaming chat
    /// responses. Each successfully parsed chunk is passed to the provided
    /// callback function.
    ///
    /// ## Arguments
    ///
    /// * `on_chunk` - Async callback function that processes each
    ///   `ChatStreamResponse` chunk
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
    ) -> impl core::future::Future<Output = crate::ZaiResult<()>> + 'a
    where
        F: FnMut(ChatStreamResponse) -> Fut + 'a,
        Fut: core::future::Future<Output = crate::ZaiResult<()>> + 'a,
    {
        async move {
            let resp = self.post().await?;
            let mut stream = resp.bytes_stream();
            let mut buf: Vec<u8> = Vec::new();

            while let Some(next) = stream.next().await {
                let bytes = match next {
                    Ok(b) => b,
                    Err(e) => {
                        return Err(crate::client::error::ZaiError::NetworkError(
                            std::sync::Arc::new(e),
                        ));
                    },
                };
                let lines = crate::model::sse_parser::extract_sse_data_lines(&mut buf, &bytes);
                for rest in lines {
                    info!("SSE data: {}", String::from_utf8_lossy(&rest));
                    if rest == b"[DONE]" {
                        return Ok(());
                    }
                    if let Ok(chunk) = serde_json::from_slice::<ChatStreamResponse>(&rest) {
                        on_chunk(chunk).await?;
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
    /// A future that resolves to a `Stream` of `Result<ChatStreamResponse>`
    /// items
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
        Output = crate::ZaiResult<
            Pin<Box<dyn Stream<Item = crate::ZaiResult<ChatStreamResponse>> + Send + 'static>>,
        >,
    > + 'a {
        async move {
            let resp = self.post().await?;
            let byte_stream = resp.bytes_stream();

            let s = byte_stream;

            let out = stream::unfold(
                (s, Vec::<u8>::new(), VecDeque::<ChatStreamResponse>::new()),
                |(mut s, mut buf, mut pending)| async move {
                    if let Some(item) = pending.pop_front() {
                        return Some((Ok(item), (s, buf, pending)));
                    }

                    loop {
                        // Need more bytes first to populate buffer
                        match s.next().await {
                            Some(Ok(bytes)) => {
                                let lines = crate::model::sse_parser::extract_sse_data_lines(
                                    &mut buf, &bytes,
                                );
                                for rest in lines {
                                    info!("SSE data: {}", String::from_utf8_lossy(&rest));
                                    if rest == b"[DONE]" {
                                        return None; // end stream gracefully
                                    }
                                    if let Ok(item) =
                                        serde_json::from_slice::<ChatStreamResponse>(&rest)
                                    {
                                        pending.push_back(item);
                                    }
                                    // skip invalid json line, continue
                                    // processing
                                    // remaining lines
                                }
                                if let Some(item) = pending.pop_front() {
                                    return Some((Ok(item), (s, buf, pending)));
                                }
                                // All lines processed but no valid
                                // ChatStreamResponse yielded,
                                // loop back to get more bytes
                            },
                            Some(Err(e)) => {
                                return Some((
                                    Err(crate::client::error::ZaiError::NetworkError(
                                        std::sync::Arc::new(e),
                                    )),
                                    (s, buf, pending),
                                ));
                            },
                            None => return None,
                        }
                    }
                },
            )
            .boxed();

            Ok(out)
        }
    }
}
