//! Fixed payload and frame limits used by transport components.
//!
//! Every constant in this module is enforced by the corresponding HTTP,
//! multipart, redaction, or realtime transport path.

/// Maximum decoded JSON request body (32 MiB).
pub const JSON_REQUEST_MAX: u64 = 32 * 1024 * 1024;
/// Maximum decoded JSON response body (32 MiB).
pub const JSON_RESPONSE_MAX: u64 = 32 * 1024 * 1024;
/// Maximum error response body read (64 KiB).
pub const ERROR_BODY_MAX: u64 = 64 * 1024;
/// Maximum decoded payload bytes in one SSE event (32 MiB).
pub const SSE_EVENT_BYTES_MAX: usize = 32 * 1024 * 1024;
/// Maximum number of `data:` fields joined into one SSE event.
pub const SSE_EVENT_DATA_LINES_MAX: usize = 4_096;
/// Maximum input slice copied into the incremental SSE parser at once.
pub const SSE_PARSE_SLICE_BYTES: usize = 64 * 1024;
/// Multipart: at most 16 file parts per request.
pub const MULTIPART_MAX_FILE_PARTS: usize = 16;
/// Multipart: maximum bytes in each individual file part (128 MiB).
pub const MULTIPART_FILE_BYTES_MAX: u64 = 128 * 1024 * 1024;
/// Multipart: non-file field bytes total (1 MiB).
pub const MULTIPART_FIELD_BYTES_MAX: u64 = 1024 * 1024;
/// WebSocket message max (8 MiB).
#[cfg(feature = "realtime")]
pub const WS_MESSAGE_MAX: u64 = 8 * 1024 * 1024;
/// WebSocket frame max (2 MiB).
#[cfg(feature = "realtime")]
pub const WS_FRAME_MAX: u64 = 2 * 1024 * 1024;
/// Realtime inbound/outbound audio frame max (4 MiB).
#[cfg(feature = "realtime")]
pub const REALTIME_AUDIO_FRAME_MAX: u64 = 4 * 1024 * 1024;
