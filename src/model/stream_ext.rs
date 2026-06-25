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
//!         Err(e) => tracing::error!("Error: {}", e),
//!     }
//! }
//! ```

use std::{collections::VecDeque, pin::Pin, sync::Arc};

use futures::{Stream, StreamExt, stream};
use tracing::trace;
use validator::Validate;

use crate::{
    client::{
        error::{ZaiError, ZaiResult},
        http::HttpClient,
    },
    model::{
        chat_base_response::ToolCallMessage, chat_stream_response::ChatStreamResponse,
        sse_parser::SseEventParser, traits::SseStreamable,
    },
};

/// Parse one completed chat SSE event.
///
/// Returns `Ok(None)` for the terminal `[DONE]` marker and a JSON error for
/// malformed event payloads.
pub fn parse_chat_stream_event(event: &[u8]) -> ZaiResult<Option<ChatStreamResponse>> {
    if event == b"[DONE]" {
        return Ok(None);
    }

    serde_json::from_slice::<ChatStreamResponse>(event)
        .map(Some)
        .map_err(|e| ZaiError::JsonError(Arc::new(e)))
}

/// Accumulates streaming chat deltas into convenient final buffers.
#[derive(Debug, Clone, Default)]
pub struct ChatStreamAccumulator {
    /// Concatenated assistant content across all chunks.
    pub content: String,
    /// Concatenated reasoning content across all chunks.
    pub reasoning_content: String,
    /// Merged tool/function calls across all chunks.
    pub tool_calls: Vec<ToolCallMessage>,
}

impl ChatStreamAccumulator {
    /// Create a new empty accumulator.
    pub fn new() -> Self {
        Self::default()
    }

    /// Fold one streaming chunk into the accumulator.
    pub fn push_chunk(&mut self, chunk: &ChatStreamResponse) {
        for choice in &chunk.choices {
            let Some(delta) = &choice.delta else {
                continue;
            };

            if let Some(content) = &delta.content {
                self.content.push_str(content);
            }
            if let Some(reasoning_content) = &delta.reasoning_content {
                self.reasoning_content.push_str(reasoning_content);
            }
            if let Some(tool_calls) = &delta.tool_calls {
                merge_tool_calls(&mut self.tool_calls, tool_calls);
            }
        }
    }
}

fn merge_tool_calls(accumulated: &mut Vec<ToolCallMessage>, deltas: &[ToolCallMessage]) {
    for (index, delta) in deltas.iter().enumerate() {
        if accumulated.len() <= index {
            accumulated.push(delta.clone());
            continue;
        }

        let current = &mut accumulated[index];
        if delta.id.is_some() {
            current.id = delta.id.clone();
        }
        if delta.type_.is_some() {
            current.type_ = delta.type_.clone();
        }
        if let Some(delta_function) = &delta.function {
            match &mut current.function {
                Some(current_function) => {
                    if delta_function.name.is_some() {
                        current_function.name = delta_function.name.clone();
                    }
                    if let Some(arguments) = &delta_function.arguments {
                        current_function
                            .arguments
                            .get_or_insert_with(String::new)
                            .push_str(arguments);
                    }
                },
                None => current.function = Some(delta_function.clone()),
            }
        }
        if delta.mcp.is_some() {
            current.mcp = delta.mcp.clone();
        }
    }
}

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
        Self::Body: Validate,
    {
        async move {
            // Field-level validation (temperature/top_p/max_tokens/…) before
            // we open the connection, mirroring the non-streaming `send()` path.
            // The stream-state guard does NOT apply here (StreamOn is expected).
            self.body().validate().map_err(ZaiError::from)?;
            let resp = self.post().await?;
            let mut stream = resp.bytes_stream();
            let mut parser = SseEventParser::new();

            while let Some(next) = stream.next().await {
                let bytes = match next {
                    Ok(b) => b,
                    Err(e) => {
                        return Err(crate::client::error::ZaiError::NetworkError(
                            std::sync::Arc::new(e),
                        ));
                    },
                };
                for event in parser.push(&bytes) {
                    // Per-chunk SSE payload logging is noisy at high token
                    // rates; keep it at `trace`.
                    trace!(parser = "sse", bytes = event.len(), "SSE chunk received");
                    match parse_chat_stream_event(&event)? {
                        Some(chunk) => on_chunk(chunk).await?,
                        None => return Ok(()),
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
    > + 'a
    where
        Self::Body: Validate,
    {
        async move {
            // Field-level validation before opening the connection, mirroring
            // the non-streaming `send()` path (stream-state guard excluded).
            self.body().validate().map_err(ZaiError::from)?;
            let resp = self.post().await?;
            let byte_stream = resp.bytes_stream();

            let s = byte_stream;

            let out = stream::unfold(
                (
                    s,
                    SseEventParser::new(),
                    VecDeque::<ZaiResult<ChatStreamResponse>>::new(),
                    false,
                ),
                |(mut s, mut parser, mut pending, done)| async move {
                    if let Some(item) = pending.pop_front() {
                        return Some((item, (s, parser, pending, done)));
                    }
                    if done {
                        return None;
                    }

                    loop {
                        // Need more bytes first to populate buffer
                        match s.next().await {
                            Some(Ok(bytes)) => {
                                let mut saw_done = done;
                                for event in parser.push(&bytes) {
                                    trace!(
                                        parser = "sse",
                                        bytes = event.len(),
                                        "SSE chunk received"
                                    );
                                    match parse_chat_stream_event(&event) {
                                        Ok(Some(item)) => pending.push_back(Ok(item)),
                                        Ok(None) => {
                                            saw_done = true;
                                            break;
                                        },
                                        Err(e) => pending.push_back(Err(e)),
                                    }
                                }
                                if let Some(item) = pending.pop_front() {
                                    return Some((item, (s, parser, pending, saw_done)));
                                }
                                if saw_done {
                                    return None;
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
                                    (s, parser, pending, done),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        chat_base_response::{ToolCallMessage, ToolFunction},
        chat_stream_response::{ChatStreamResponse, Delta, StreamChoice},
    };

    fn chunk_with_delta(delta: Delta) -> ChatStreamResponse {
        ChatStreamResponse {
            id: Some("test".to_string()),
            created: Some(1),
            model: Some("glm-test".to_string()),
            choices: vec![StreamChoice {
                index: Some(0),
                delta: Some(delta),
                finish_reason: None,
            }],
            usage: None,
        }
    }

    #[test]
    fn parse_done_event() {
        assert!(parse_chat_stream_event(b"[DONE]").unwrap().is_none());
    }

    #[test]
    fn parse_invalid_event_returns_json_error() {
        let err = parse_chat_stream_event(b"{not-json").unwrap_err();
        assert!(matches!(err, ZaiError::JsonError(_)));
    }

    #[test]
    fn accumulator_appends_content_reasoning_and_tool_arguments() {
        let mut acc = ChatStreamAccumulator::new();

        acc.push_chunk(&chunk_with_delta(Delta {
            role: Some("assistant".to_string()),
            content: Some("hel".to_string()),
            reasoning_content: Some("think ".to_string()),
            tool_calls: Some(vec![ToolCallMessage {
                id: Some("call_1".to_string()),
                type_: Some("function".to_string()),
                function: Some(ToolFunction {
                    name: Some("search".to_string()),
                    arguments: Some("{\"q\":\"ru".to_string()),
                }),
                mcp: None,
            }]),
        }));

        acc.push_chunk(&chunk_with_delta(Delta {
            role: None,
            content: Some("lo".to_string()),
            reasoning_content: Some("done".to_string()),
            tool_calls: Some(vec![ToolCallMessage {
                id: None,
                type_: None,
                function: Some(ToolFunction {
                    name: None,
                    arguments: Some("st\"}".to_string()),
                }),
                mcp: None,
            }]),
        }));

        assert_eq!(acc.content, "hello");
        assert_eq!(acc.reasoning_content, "think done");
        assert_eq!(
            acc.tool_calls[0]
                .function
                .as_ref()
                .and_then(|function| function.arguments.as_deref()),
            Some("{\"q\":\"rust\"}")
        );
    }
}
