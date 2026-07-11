//! Streaming extension traits (P05 cleanup: HttpClient trait removed).
//!
//! The SSE streaming methods (`stream_for_each`, `stream_sse_for_each`,
//! `to_stream`) were default methods on traits that required `HttpClient`.
//! Since `HttpClient` is removed in 0.5, these traits are now empty marker
//! traits. The full SSE streaming path will be rebuilt in a future point
//! release via the new `Transport` + `ZaiClient` architecture.

use crate::model::traits::SseStreamable;

/// Marker trait for types that supported SSE streaming in 0.4.
/// Retained as an empty trait for backward compatibility.
pub trait StreamChatLikeExt: SseStreamable {}
