//! Transport retry-safety, status classification, and backoff tests.
//!
//! Exercises the retry-safety classification, the retryable-status set and the
//! full-jitter backoff directly through their pure helper functions.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use zai_rs::client::RetryOverride;
use zai_rs::client::transport::retry::{
    JitterSource, NON_RETRYABLE_QUOTA_CODES, NON_RETRYABLE_VALIDATION_CODES, RETRYABLE_STATUSES,
    RetrySafety, backoff_delay, full_jitter_cap, is_retryable_outcome, parse_retry_after,
    reconcile_retry_after,
};

#[test]
fn retry_safety_matrix_is_fixed_by_method() {
    for m in ["GET", "HEAD", "OPTIONS", "PUT", "DELETE"] {
        assert_eq!(
            RetrySafety::for_method(m),
            RetrySafety::Idempotent,
            "{m} must be Idempotent"
        );
    }
    for m in ["POST", "PATCH"] {
        assert_eq!(
            RetrySafety::for_method(m),
            RetrySafety::NonIdempotent,
            "{m} must be NonIdempotent"
        );
    }
}

#[test]
fn override_is_the_only_escape_hatch() {
    // POST is NonIdempotent; AssumeIdempotent flips it.
    let s = RetrySafety::for_method("POST").effective(Some(RetryOverride::AssumeIdempotent));
    assert_eq!(s, RetrySafety::Idempotent);
    // Without override it stays NonIdempotent.
    assert_eq!(
        RetrySafety::for_method("POST").effective(None),
        RetrySafety::NonIdempotent
    );
}

#[test]
fn retryable_status_set_excludes_501_505() {
    for s in [408, 425, 429, 500, 502, 503, 504] {
        assert!(RETRYABLE_STATUSES.contains(&s), "{s} should be retryable");
        assert!(is_retryable_outcome(s, None), "{s} should retry");
    }
    // 501/505 excluded.
    assert!(!RETRYABLE_STATUSES.contains(&501));
    assert!(!RETRYABLE_STATUSES.contains(&505));
    // 4xx other than the retry set excluded.
    assert!(!is_retryable_outcome(400, None));
    assert!(!is_retryable_outcome(404, None));
}

#[test]
fn quota_and_validation_codes_override_status() {
    // Every non-retryable quota code at a 429 status is NOT retried.
    for code in NON_RETRYABLE_QUOTA_CODES {
        assert!(
            !is_retryable_outcome(429, Some(*code)),
            "quota code {code} at 429 must not retry"
        );
    }
    // Every validation code at 429 is NOT retried.
    for code in NON_RETRYABLE_VALIDATION_CODES {
        assert!(
            !is_retryable_outcome(429, Some(*code)),
            "validation code {code} at 429 must not retry"
        );
    }
    // 1302/1305 ARE retryable at 429/503 (rate-limit retryable).
    assert!(is_retryable_outcome(429, Some(1302)));
    assert!(is_retryable_outcome(503, Some(1305)));
}

#[test]
fn full_jitter_progression_and_cap() {
    // 200ms * 2^n capped at 8s.
    assert_eq!(full_jitter_cap(0), Duration::from_millis(200));
    assert_eq!(full_jitter_cap(1), Duration::from_millis(400));
    assert_eq!(full_jitter_cap(2), Duration::from_millis(800));
    assert_eq!(full_jitter_cap(3), Duration::from_millis(1600));
    assert_eq!(full_jitter_cap(4), Duration::from_millis(3200));
    assert_eq!(full_jitter_cap(5), Duration::from_millis(6400));
    assert_eq!(full_jitter_cap(6), Duration::from_secs(8));
    assert_eq!(full_jitter_cap(20), Duration::from_secs(8));
}

#[test]
fn backoff_with_injected_jitter_never_exceeds_cap() {
    struct Zero;
    impl JitterSource for Zero {
        fn jitter(&self, _upper: Duration) -> Duration {
            Duration::ZERO
        }
    }
    struct Max;
    impl JitterSource for Max {
        fn jitter(&self, upper: Duration) -> Duration {
            upper
        }
    }
    for n in 0..8 {
        assert_eq!(backoff_delay(n, &Zero), Duration::ZERO);
        assert_eq!(backoff_delay(n, &Max), full_jitter_cap(n));
    }
}

#[test]
fn retry_after_parsing_and_reconciliation() {
    assert_eq!(parse_retry_after("120"), Some(Duration::from_secs(120)));
    assert_eq!(parse_retry_after("0"), None);
    assert_eq!(parse_retry_after("garbage"), None);
    // hint >= computed wins; shorter does not.
    let computed = Duration::from_millis(500);
    assert_eq!(
        reconcile_retry_after(Some(Duration::from_secs(5)), computed),
        Duration::from_secs(5)
    );
    assert_eq!(
        reconcile_retry_after(Some(Duration::from_millis(100)), computed),
        computed
    );
    assert_eq!(reconcile_retry_after(None, computed), computed);
}

#[test]
fn deterministic_jitter_makes_backoff_reproducible() {
    struct Step(AtomicU64);
    impl JitterSource for Step {
        fn jitter(&self, upper: Duration) -> Duration {
            let n = self.0.fetch_add(1, Ordering::SeqCst);
            Duration::from_millis(((n + 1) * 10).min(upper.as_millis() as u64))
        }
    }
    let j = Step(AtomicU64::new(0));
    let d0 = backoff_delay(0, &j);
    let j2 = Step(AtomicU64::new(0));
    let d0_again = backoff_delay(0, &j2);
    assert_eq!(d0, d0_again, "same seed → same delay");
}
