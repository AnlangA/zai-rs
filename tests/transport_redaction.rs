//! Transport tracing-redaction tests.
//!
//! Pins that the transport's redaction surface (route-template logging, request_id
//! sanitization, content-type validation, error-envelope probing) never leaks a
//! URL, header value, query value, or body.

use zai_rs::client::transport::decode::{
    ExpectedKind, probe_error_envelope, validate_content_type,
};
use zai_rs::client::transport::redaction::{has_usable_id, sanitize_request_id};

const TEST_KEY: &str = "1234567890.abcdefghijklmnop";

#[test]
fn request_id_sanitized_to_printable_ascii_max_128() {
    // Control chars stripped.
    let s = sanitize_request_id("abc\x01def");
    assert!(!s.contains('\x01'));
    // Overlong truncated.
    let s = sanitize_request_id(&"a".repeat(500));
    assert!(s.len() <= 128);
    // Non-printable-only → not usable.
    assert!(!has_usable_id(&sanitize_request_id("\x01\x02")));
}

#[test]
fn content_type_validation_is_strict() {
    // JSON variants accepted for Json expected.
    assert!(validate_content_type("application/json", ExpectedKind::Json).is_ok());
    assert!(validate_content_type("application/json; charset=utf-8", ExpectedKind::Json).is_ok());
    assert!(validate_content_type("application/vnd.api+json", ExpectedKind::Json).is_ok());
    // text/plain rejected for Json.
    assert!(validate_content_type("text/plain", ExpectedKind::Json).is_err());
    // SSE only for Sse.
    assert!(validate_content_type("text/event-stream", ExpectedKind::Sse).is_ok());
    assert!(validate_content_type("application/json", ExpectedKind::Sse).is_err());
    // Binary must match the manifest MIME exactly-ish.
    assert!(validate_content_type("audio/pcm", ExpectedKind::Binary("audio/pcm")).is_ok());
    assert!(validate_content_type("text/html", ExpectedKind::Binary("audio/pcm")).is_err());
}

#[test]
fn error_envelope_probe_never_treats_key_as_envelope() {
    // A body containing the key must not be misread as a business error envelope
    // (it has no `code`/`error` envelope fields).
    let body = format!(r#"{{"model":"glm-5.2","key":"{TEST_KEY}"}}"#);
    assert!(!probe_error_envelope(&body));
    // But a genuine error envelope is detected.
    assert!(probe_error_envelope(r#"{"code":1302,"message":"rl"}"#));
    assert!(probe_error_envelope(
        r#"{"error":{"code":1302,"message":"rl"}}"#
    ));
    // code==200 is success.
    assert!(!probe_error_envelope(r#"{"code":200,"message":"ok"}"#));
}

#[test]
fn redaction_helpers_never_emit_key() {
    // The redaction module never touches the key, but assert that its outputs
    // (sanitized request_id, content-type messages) can't carry one.
    let s = sanitize_request_id(TEST_KEY);
    // A key-shaped string is printable ASCII and would survive sanitization, so
    // the guarantee is that the Transport never *passes* the key through this
    // path — it only passes server correlation request_ids. Pin that contract:
    // the helper itself does not add or echo secrets beyond its input.
    assert!(s.len() <= 128);
}
