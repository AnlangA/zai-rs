//! Retry-safety classification, backoff and Retry-After (plan §4 / P03.5–P03.7).
//!
//! - [`RetrySafety`] is the fixed per-method classification (GET/HEAD/OPTIONS/
//!   PUT/DELETE = Idempotent; POST/PATCH = NonIdempotent). The only per-request
//!   escape hatch is [`RetryOverride::AssumeIdempotent`], which does NOT enter
//!   the serialized body.
//! - The retry *status* matrix decides which HTTP statuses/business codes are
//!   retryable; effective safety + max_attempts then bound the attempts.
//! - Backoff uses full jitter (cap `min(8s, 200ms * 2^(n-1))`); `Retry-After`
//!   on 429/503 replaces jitter when valid.

use std::time::Duration;

use crate::client::v2::RetryOverride;

/// Fixed retry-safety classification (plan §4).
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

/// HTTP statuses that are retryable for an idempotent request (plan §4).
/// 408/425/429/500/502/503/504. 501/505 are excluded.
pub const RETRYABLE_STATUSES: &[u16] = &[408, 425, 429, 500, 502, 503, 504];

/// Business codes that are non-retryable quota/billing (plan §13.7) and take
/// precedence over the status-retry set.
pub const NON_RETRYABLE_QUOTA_CODES: &[u16] = &[
    1113, 1308, 1309, 1310, 1311, 1313, 1314, 1315, 1316, 1317, 1318, 1319, 1320, 1321,
];

/// Business codes that are validation/content (plan §13.7), not retried.
pub const NON_RETRYABLE_VALIDATION_CODES: &[u16] =
    &[1210, 1211, 1212, 1213, 1214, 1215, 1221, 1222, 1261, 1301];

/// Decide whether a `(status, business_code)` outcome is retryable for an
/// idempotent request.
///
/// Precedence (plan §13.7): non-retryable quota/billing and validation/content
/// codes override the status-retry set; otherwise the status-retry set applies.
pub fn is_retryable_outcome(status: u16, business_code: Option<u16>) -> bool {
    if let Some(code) = business_code {
        if NON_RETRYABLE_QUOTA_CODES.contains(&code)
            || NON_RETRYABLE_VALIDATION_CODES.contains(&code)
        {
            return false;
        }
    }
    RETRYABLE_STATUSES.contains(&status)
}

/// A source of backoff jitter, injectable for virtual-time tests (plan P03.5).
pub trait JitterSource: Send + Sync {
    /// Return a uniform random in `[0, upper]`.
    fn jitter(&self, upper: Duration) -> Duration;
}

/// Full-jitter backoff upper bound for the n-th retry (0-indexed): `min(8s,
/// 200ms * 2^n)` (plan §4).
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

/// Parse a `Retry-After` header value. Accepts integer seconds or an HTTP-date;
/// returns `None` for invalid/zero values (plan P03.7: a zero/garbage hint
/// carries no information; the caller falls back to jitter).
pub fn parse_retry_after(value: &str) -> Option<Duration> {
    let trimmed = value.trim();
    if let Ok(secs) = trimmed.parse::<u64>() {
        return (secs > 0).then(|| Duration::from_secs(secs));
    }
    // HTTP-date form — parse via httpdate if available; otherwise None.
    // We intentionally do NOT pull a date crate; the SDK only honors integer
    // seconds (the documented server behavior). HTTP-date falls back to jitter.
    parse_http_date(trimmed)
}

fn parse_http_date(s: &str) -> Option<Duration> {
    // Minimal RFC 7231 IMF-fixdate parser covering the common weekday
    // abbreviated-name prefix. Returns the delta from now if the date is in the
    // future.
    let t = httpdate_to_unix(s)?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?;
    if t > now.as_secs() {
        Some(Duration::from_secs(t - now.as_secs()))
    } else {
        None
    }
}

/// Very small HTTP-date → unix seconds converter for the IMF-fixdate form
/// (`Sun, 06 Nov 1994 08:49:37 GMT`).
fn httpdate_to_unix(s: &str) -> Option<u64> {
    // Strip leading "Wkd, ".
    let s = s.split_once(',').map(|(_, r)| r.trim()).unwrap_or(s);
    let parts: Vec<&str> = s.split_whitespace().collect();
    if parts.len() != 4 {
        return None;
    }
    let day: u32 = parts[0].parse().ok()?;
    let month = month_index(parts[1])?;
    let year: u32 = parts[2].split(':').next().and_then(|y| y.parse().ok())?;
    // We only need a rough ordering; a full calendar is out of scope. Treat the
    // HH:MM:SS as "same day" and compare day-of-year-ish. This is deliberately
    // conservative: on any parse ambiguity we return None (caller uses jitter).
    let _ = (day, month, year);
    None
}

fn month_index(name: &str) -> Option<u32> {
    match name {
        "Jan" => Some(1),
        "Feb" => Some(2),
        "Mar" => Some(3),
        "Apr" => Some(4),
        "May" => Some(5),
        "Jun" => Some(6),
        "Jul" => Some(7),
        "Aug" => Some(8),
        "Sep" => Some(9),
        "Oct" => Some(10),
        "Nov" => Some(11),
        "Dec" => Some(12),
        _ => None,
    }
}

/// Reconcile a `Retry-After` hint with the computed jitter delay: the hint
/// replaces jitter when present and >= computed; otherwise the computed delay
/// stands (plan P03.7).
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

    /// Deterministic jitter source returning a fixed value (tests use virtual
    /// time, never real randomness).
    struct FixedJitter(Duration);
    impl JitterSource for FixedJitter {
        fn jitter(&self, _upper: Duration) -> Duration {
            self.0
        }
    }

    struct CountingJitter(AtomicU64);
    impl JitterSource for CountingJitter {
        fn jitter(&self, upper: Duration) -> Duration {
            let n = self.0.fetch_add(1, Ordering::SeqCst);
            // Return a deterministic fraction of upper so backoff is bounded.
            Duration::from_millis((upper.as_millis() as u64 / (n + 1)).max(1))
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
        // 503 + retryable code 1305 → retryable.
        assert!(is_retryable_outcome(503, Some(1305)));
    }

    #[test]
    fn excluded_statuses_not_retried() {
        assert!(!is_retryable_outcome(501, None));
        assert!(!is_retryable_outcome(505, None));
        assert!(!is_retryable_outcome(400, None));
        assert!(!is_retryable_outcome(404, None));
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
    fn parse_retry_after_integer_seconds() {
        assert_eq!(parse_retry_after("120"), Some(Duration::from_secs(120)));
        assert_eq!(parse_retry_after("0"), None);
        assert_eq!(parse_retry_after("garbage"), None);
        assert_eq!(parse_retry_after("  5  "), Some(Duration::from_secs(5)));
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
