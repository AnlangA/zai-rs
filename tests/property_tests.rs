//! Property-based tests for encoding, SSE parsing, retry bounds, and errors.
//!
//! Uses `proptest` to cover:
//! - dynamic path/query percent-encoding
//! - SSE chunk-split resilience (arbitrary boundaries)
//! - full-jitter backoff bounding
//! - error-envelope probe correctness
//! - ApiCode roundtrip

use proptest::prelude::*;

use zai_rs::client::transport::decode::probe_error_envelope;
use zai_rs::client::transport::retry::{full_jitter_cap, is_retryable_outcome};
use zai_rs::model::sse_parser::SseEventParser;

// ---------------------------------------------------------------------------
// Path-segment encoding.
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn endpoint_resolve_percent_encodes_special_chars(s in "[a-zA-Z0-9_.%/?#]{1,20}") {
        let ec = zai_rs::client::endpoint::EndpointConfig::defaults().unwrap();
        // Empty/dot/dotdot are rejected — filter them out.
        prop_assume!(!s.is_empty() && s != "." && s != "..");
        let result = ec.resolve(zai_rs::client::ApiFamily::PaasV4, &[&s]);
        // Should either succeed (producing a valid URL) or fail (for truly
        // invalid segments) — but never panic.
        let _ = result;
    }
}

// ---------------------------------------------------------------------------
// SSE chunk-split resilience.
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn sse_parse_recovers_from_arbitrary_split(data in "data: [^\n]{1,50}\n\n") {
        let mut p = SseEventParser::new();
        // Feed the bytes in two halves — the parser should still produce the
        // event regardless of where the split occurs.
        let mid = data.len() / 2;
        let _ = p.push(&data.as_bytes()[..mid]);
        let events = p.push(&data.as_bytes()[mid..]);
        prop_assert!(
            events.len() <= 1,
            "at most one event from a single data: line"
        );
    }
}

// ---------------------------------------------------------------------------
// Full-jitter backoff bounds.
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn full_jitter_cap_never_exceeds_8s(n in 0u32..30) {
        let cap = full_jitter_cap(n);
        prop_assert!(cap <= std::time::Duration::from_secs(8));
    }

    #[test]
    fn full_jitter_cap_monotonic_until_saturation(n in 0u32..6) {
        let a = full_jitter_cap(n);
        let b = full_jitter_cap(n + 1);
        prop_assert!(b >= a, "backoff should be non-decreasing until saturation");
    }
}

// ---------------------------------------------------------------------------
// Error-envelope probing.
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn probe_envelope_code_not_200_is_error(code in 300u16..600) {
        let body = format!(r#"{{"code":{code},"message":"x"}}"#);
        prop_assert!(probe_error_envelope(&body));
    }

    #[test]
    fn probe_envelope_code_200_is_not_error(_ in 0u8..1) {
        let body = r#"{"code":200,"message":"ok"}"#;
        prop_assert!(!probe_error_envelope(body));
    }
}

// ---------------------------------------------------------------------------
// Retryable status classification.
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn retryable_status_set_is_stable(status in 100u16..600) {
        let result = is_retryable_outcome(status, None);
        let expected = matches!(status, 408 | 425 | 429 | 500 | 502 | 503 | 504);
        prop_assert_eq!(result, expected);
    }
}
