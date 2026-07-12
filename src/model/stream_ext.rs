//! Legacy streaming extension traits retained for source compatibility.
//!
//! These marker traits contain no streaming methods and have no implementations
//! in this crate; they are not an operational streaming API.

use crate::model::traits::SseStreamable;

/// Marker trait for types that supported SSE streaming in 0.4.
/// Retained as an empty trait for backward compatibility.
pub trait StreamChatLikeExt: SseStreamable {}
