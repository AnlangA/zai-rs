//! Fixed payload limits (plan §4 / P03.8).
//!
//! Every limit is both the default AND the hard upper bound — the builder only
//! ever lets callers *lower* them. These constants are the single source of
//! truth for the Transport's size enforcement.

/// Maximum decoded JSON request body (32 MiB).
pub const JSON_REQUEST_MAX: u64 = 32 * 1024 * 1024;
/// Maximum decoded JSON response body (32 MiB).
pub const JSON_RESPONSE_MAX: u64 = 32 * 1024 * 1024;
/// Maximum error response body read (64 KiB).
pub const ERROR_BODY_MAX: u64 = 64 * 1024;
/// Maximum a single SSE line / event / unparsed buffer may grow (1 MiB each).
pub const SSE_LINE_MAX: u64 = 1024 * 1024;
pub const SSE_EVENT_MAX: u64 = 1024 * 1024;
pub const SSE_BUFFER_MAX: u64 = 1024 * 1024;
/// Multipart: at most 16 file parts per request.
pub const MULTIPART_MAX_FILE_PARTS: usize = 16;
/// Multipart: file bytes total across parts (128 MiB).
pub const MULTIPART_FILE_BYTES_MAX: u64 = 128 * 1024 * 1024;
/// Multipart: non-file field bytes total (1 MiB).
pub const MULTIPART_FIELD_BYTES_MAX: u64 = 1024 * 1024;
/// WebSocket message max (8 MiB).
pub const WS_MESSAGE_MAX: u64 = 8 * 1024 * 1024;
/// WebSocket frame max (2 MiB).
pub const WS_FRAME_MAX: u64 = 2 * 1024 * 1024;
/// Realtime inbound/outbound audio frame max (4 MiB).
pub const REALTIME_AUDIO_FRAME_MAX: u64 = 4 * 1024 * 1024;
/// Correlation request_id max retained length (128 bytes printable ASCII).
pub const REQUEST_ID_MAX: usize = 128;
/// ApiCode::Text max length (128 bytes).
pub const API_CODE_TEXT_MAX: usize = 128;

/// A clamped limit value: never above the hard cap, optionally lowered.
#[derive(Debug, Clone, Copy)]
pub struct Limit {
    pub bytes: u64,
}

impl Limit {
    /// Clamp a caller-requested value to `hard` (the value above which the
    /// builder rejects). Returns the original if `<= hard`.
    pub fn clamp(requested: u64, hard: u64) -> Self {
        Self {
            bytes: requested.min(hard),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_limits_match_plan() {
        assert_eq!(JSON_REQUEST_MAX, 32 * 1024 * 1024);
        assert_eq!(JSON_RESPONSE_MAX, 32 * 1024 * 1024);
        assert_eq!(ERROR_BODY_MAX, 64 * 1024);
        assert_eq!(SSE_LINE_MAX, 1024 * 1024);
        assert_eq!(MULTIPART_FILE_BYTES_MAX, 128 * 1024 * 1024);
        assert_eq!(WS_MESSAGE_MAX, 8 * 1024 * 1024);
        assert_eq!(REALTIME_AUDIO_FRAME_MAX, 4 * 1024 * 1024);
    }

    #[test]
    fn clamp_never_exceeds_hard_cap() {
        assert_eq!(Limit::clamp(100, 50).bytes, 50);
        assert_eq!(Limit::clamp(50, 100).bytes, 50);
        assert_eq!(Limit::clamp(100, 100).bytes, 100);
    }
}
