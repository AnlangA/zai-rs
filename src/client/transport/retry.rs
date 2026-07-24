//! Retry-safety classification, backoff, and `Retry-After` handling.
//!
//! - [`RetrySafety`] is the fixed per-method classification (GET/HEAD/OPTIONS/
//!   PUT/DELETE = Idempotent; POST/PATCH = NonIdempotent). The only per-request
//!   escape hatch is [`RetryOverride::AssumeIdempotent`], which does NOT enter
//!   the serialized body.
//! - The retry *status* matrix decides which HTTP statuses/business codes are
//!   retryable; effective safety + max_attempts then bound the attempts.
//! - Backoff uses full jitter (cap `min(8s, 200ms * 2^n)` for zero-based retry
//!   index `n`). A positive integer `Retry-After` hint replaces the jitter delay
//!   only when it is at least as long as that delay. Both standard
//!   delta-seconds and IMF-fixdate values are accepted.

use std::time::Duration;

use crate::client::RetryOverride;

/// Fixed retry-safety classification derived from the HTTP method.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetrySafety {
    /// GET/HEAD/OPTIONS/PUT/DELETE — safe to retry on transient errors.
    Idempotent,
    /// POST/PATCH — NOT retried unless overridden.
    NonIdempotent,
}

impl RetrySafety {
    /// The fixed classification for an HTTP method.
    pub fn for_method(method: &str) -> Self {
        match method {
            "GET" | "HEAD" | "OPTIONS" | "PUT" | "DELETE" => RetrySafety::Idempotent,
            _ => RetrySafety::NonIdempotent,
        }
    }

    /// Effective safety given an optional per-request override.
    pub fn effective(self, override_: Option<RetryOverride>) -> Self {
        match override_ {
            Some(RetryOverride::AssumeIdempotent) => RetrySafety::Idempotent,
            None => self,
        }
    }
}

/// HTTP statuses that are retryable for an idempotent request.
/// 408/425/429/500/502/503/504. 501/505 are excluded.
pub const RETRYABLE_STATUSES: &[u16] = &[408, 425, 429, 500, 502, 503, 504];

/// Business codes treated as non-retryable quota or billing failures.
///
/// These codes take precedence over the status retry set.
pub const NON_RETRYABLE_QUOTA_CODES: &[u16] = &[
    1113, 1308, 1309, 1310, 1311, 1313, 1314, 1315, 1316, 1317, 1318, 1319, 1320, 1321,
];

/// Business codes treated as non-retryable validation or content failures.
pub const NON_RETRYABLE_VALIDATION_CODES: &[u16] =
    &[1210, 1211, 1212, 1213, 1214, 1215, 1221, 1222, 1261, 1301];

/// Business codes that identify transient upstream execution failures.
pub const RETRYABLE_SERVER_CODES: &[u16] = &[1200, 1230, 1234];

/// Business codes for temporary rate limits that are safe to retry when the
/// request itself is idempotent.
pub const RETRYABLE_RATE_CODES: &[u16] = &[1302, 1305];

/// Decide whether a `(status, business_code)` outcome is retryable for an
/// idempotent request.
///
/// Non-retryable quota/billing and validation/content codes override every
/// retry signal. Documented server-side business errors are retryable even when
/// an intermediary returned an unusual status; otherwise the HTTP status set
/// applies.
pub fn is_retryable_outcome(status: u16, business_code: Option<u16>) -> bool {
    if let Some(code) = business_code
        && (NON_RETRYABLE_QUOTA_CODES.contains(&code)
            || NON_RETRYABLE_VALIDATION_CODES.contains(&code))
    {
        return false;
    }
    if business_code.is_some_and(|code| {
        RETRYABLE_RATE_CODES.contains(&code) || RETRYABLE_SERVER_CODES.contains(&code)
    }) {
        return true;
    }
    RETRYABLE_STATUSES.contains(&status)
}

/// A source of backoff jitter, injectable for deterministic tests.
pub trait JitterSource: Send + Sync {
    /// Choose a delay in the inclusive range `[0, upper]`.
    fn jitter(&self, upper: Duration) -> Duration;
}

/// Full-jitter backoff upper bound for the n-th retry (0-indexed): `min(8s,
/// 200ms * 2^n)`.
pub fn full_jitter_cap(retry_index: u32) -> Duration {
    let base_ms: u64 = 200;
    let cap_ms: u64 = 8_000;
    // 200ms * 2^n, capped at 8s. Saturating shifts guard against overflow.
    let scaled = base_ms.saturating_mul(1u64 << retry_index.min(20));
    Duration::from_millis(scaled.min(cap_ms))
}

/// Compute the backoff delay for the n-th retry, applying full jitter via
/// `jitter`.
pub fn backoff_delay(retry_index: u32, jitter: &dyn JitterSource) -> Duration {
    jitter.jitter(full_jitter_cap(retry_index))
}

/// Parse a `Retry-After` header as positive delta-seconds or IMF-fixdate.
///
/// Zero, expired dates, and malformed values return `None`, which makes the
/// caller fall back to jitter.
pub fn parse_retry_after(value: &str) -> Option<Duration> {
    parse_retry_after_at(value, chrono::Utc::now())
}

fn parse_retry_after_at(value: &str, now: chrono::DateTime<chrono::Utc>) -> Option<Duration> {
    let trimmed = value.trim();
    if let Some(seconds) = trimmed.parse::<u64>().ok().filter(|seconds| *seconds > 0) {
        return Some(Duration::from_secs(seconds));
    }

    let retry_at = chrono::DateTime::parse_from_rfc2822(trimmed)
        .ok()?
        .with_timezone(&chrono::Utc);
    (retry_at - now)
        .to_std()
        .ok()
        .filter(|duration| !duration.is_zero())
}

/// Reconcile a `Retry-After` hint with the computed jitter delay: the hint
/// replaces jitter when present and >= computed; otherwise the computed delay
/// stands.
pub fn reconcile_retry_after(hint: Option<Duration>, computed: Duration) -> Duration {
    match hint {
        Some(h) if h >= computed => h,
        _ => computed,
    }

    // Note: if `hint >= remaining deadline` the caller returns Timeout directly
    // (handled in the Transport loop, not here, since the deadline is needed).
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    struct CountingJitter(AtomicU64);
    impl JitterSource for CountingJitter {
        fn jitter(&self, upper: Duration) -> Duration {
            let n = self.0.fetch_add(1, Ordering::SeqCst);
            // Return a deterministic fraction of upper so backoff is bounded.
            upper / u32::try_from(n.saturating_add(1)).unwrap_or(u32::MAX)
        }
    }

    #[test]
    fn method_classification_is_fixed() {
        assert_eq!(RetrySafety::for_method("GET"), RetrySafety::Idempotent);
        assert_eq!(RetrySafety::for_method("PUT"), RetrySafety::Idempotent);
        assert_eq!(RetrySafety::for_method("DELETE"), RetrySafety::Idempotent);
        assert_eq!(RetrySafety::for_method("POST"), RetrySafety::NonIdempotent);
        assert_eq!(RetrySafety::for_method("PATCH"), RetrySafety::NonIdempotent);
    }

    #[test]
    fn override_makes_post_idempotent() {
        let s = RetrySafety::for_method("POST").effective(Some(RetryOverride::AssumeIdempotent));
        assert_eq!(s, RetrySafety::Idempotent);
    }

    #[test]
    fn non_retryable_codes_override_status() {
        // 429 + business 1113 (quota) → NOT retryable despite 429.
        assert!(!is_retryable_outcome(429, Some(1113)));
        // 429 + business 1210 (validation) → not retryable.
        assert!(!is_retryable_outcome(429, Some(1210)));
        // 429 + no business code → retryable.
        assert!(is_retryable_outcome(429, None));
        // A retryable rate code is sufficient even if a proxy normalized the
        // HTTP status away from 429.
        for code in RETRYABLE_RATE_CODES {
            assert!(is_retryable_outcome(200, Some(*code)));
            assert!(is_retryable_outcome(400, Some(*code)));
        }
        // Server business errors remain retryable even if a proxy normalized
        // the HTTP status.
        for code in RETRYABLE_SERVER_CODES {
            assert!(is_retryable_outcome(200, Some(*code)));
        }
    }

    #[test]
    fn excluded_statuses_not_retried() {
        assert!(!is_retryable_outcome(501, None));
        assert!(!is_retryable_outcome(505, None));
        assert!(!is_retryable_outcome(400, None));
        assert!(!is_retryable_outcome(404, None));

        for status in 100_u16..600 {
            assert_eq!(
                is_retryable_outcome(status, None),
                matches!(status, 408 | 425 | 429 | 500 | 502 | 503 | 504),
                "HTTP {status}"
            );
        }
    }

    #[test]
    fn full_jitter_cap_saturates_at_8s() {
        assert_eq!(full_jitter_cap(0), Duration::from_millis(200));
        assert_eq!(full_jitter_cap(1), Duration::from_millis(400));
        assert_eq!(full_jitter_cap(2), Duration::from_millis(800));
        assert_eq!(full_jitter_cap(5), Duration::from_millis(6400));
        // 2^6 * 200ms = 12800ms → capped to 8s.
        assert_eq!(full_jitter_cap(6), Duration::from_secs(8));
        // Saturation: huge index does not overflow.
        assert_eq!(full_jitter_cap(100), Duration::from_secs(8));
        for retry in 0..30 {
            assert!(full_jitter_cap(retry) <= Duration::from_secs(8));
        }
        for retry in 0..6 {
            assert!(full_jitter_cap(retry + 1) >= full_jitter_cap(retry));
        }
    }

    #[test]
    fn backoff_is_bounded_by_cap() {
        let j = CountingJitter(AtomicU64::new(0));
        for n in 0..10 {
            let d = backoff_delay(n, &j);
            assert!(d <= full_jitter_cap(n), "retry {n} delay {d:?} exceeds cap");
        }
    }

    #[test]
    fn parse_retry_after_delta_seconds() {
        assert_eq!(parse_retry_after("120"), Some(Duration::from_secs(120)));
        assert_eq!(parse_retry_after("0"), None);
        assert_eq!(parse_retry_after("garbage"), None);
        assert_eq!(parse_retry_after("  5  "), Some(Duration::from_secs(5)));
    }

    #[test]
    fn parse_retry_after_http_date() {
        let now = chrono::DateTime::parse_from_rfc2822("Sun, 06 Nov 1994 08:47:37 GMT")
            .unwrap()
            .with_timezone(&chrono::Utc);
        assert_eq!(
            parse_retry_after_at("Sun, 06 Nov 1994 08:49:37 GMT", now),
            Some(Duration::from_secs(120))
        );
        assert_eq!(
            parse_retry_after_at("Sun, 06 Nov 1994 08:47:37 GMT", now),
            None
        );
        assert_eq!(
            parse_retry_after_at("Sun, 06 Nov 1994 08:40:00 GMT", now),
            None
        );
    }

    #[test]
    fn reconcile_takes_larger_hint() {
        let computed = Duration::from_secs(2);
        assert_eq!(reconcile_retry_after(None, computed), computed);
        assert_eq!(
            reconcile_retry_after(Some(Duration::from_secs(10)), computed),
            Duration::from_secs(10)
        );
        assert_eq!(
            reconcile_retry_after(Some(Duration::from_secs(1)), computed),
            computed
        );
    }
}
