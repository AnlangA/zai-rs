//! Payload-size limit tests for the HTTP transport.

use zai_rs::client::transport::limits::*;

#[test]
fn json_limits_match_plan_fixed_values() {
    assert_eq!(JSON_REQUEST_MAX, 32 * 1024 * 1024);
    assert_eq!(JSON_RESPONSE_MAX, 32 * 1024 * 1024);
    assert_eq!(ERROR_BODY_MAX, 64 * 1024);
}

#[test]
fn sse_limits_match_plan() {
    assert_eq!(SSE_LINE_MAX, 1024 * 1024);
    assert_eq!(SSE_EVENT_MAX, 1024 * 1024);
    assert_eq!(SSE_BUFFER_MAX, 1024 * 1024);
}

#[test]
fn multipart_limits_match_plan() {
    assert_eq!(MULTIPART_MAX_FILE_PARTS, 16);
    assert_eq!(MULTIPART_FILE_BYTES_MAX, 128 * 1024 * 1024);
    assert_eq!(MULTIPART_FIELD_BYTES_MAX, 1024 * 1024);
}

#[test]
fn websocket_and_realtime_limits_match_plan() {
    assert_eq!(WS_MESSAGE_MAX, 8 * 1024 * 1024);
    assert_eq!(WS_FRAME_MAX, 2 * 1024 * 1024);
    assert_eq!(REALTIME_AUDIO_FRAME_MAX, 4 * 1024 * 1024);
}

#[test]
fn clamp_only_lowers() {
    assert_eq!(Limit::clamp(100, 50).bytes, 50);
    assert_eq!(Limit::clamp(30, 50).bytes, 30);
    assert_eq!(Limit::clamp(50, 50).bytes, 50);
}

#[test]
fn request_id_and_code_text_caps() {
    assert_eq!(REQUEST_ID_MAX, 128);
    assert_eq!(API_CODE_TEXT_MAX, 128);
}
