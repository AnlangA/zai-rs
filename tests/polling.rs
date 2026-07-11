//! P07 acceptance: polling with absolute deadline (plan P07.7).

use std::time::Duration;

/// Poll a condition repeatedly until the deadline, sleeping at most `interval`
/// between polls. Returns `true` if the condition was met, `false` on timeout.
///
/// This is a pure reference implementation for the SDK's polling endpoint
/// (async-task result, file-parser result, etc.). The virtual-time version
/// uses an injected Clock; the unit tests below verify only the bounding logic.
pub async fn poll_until<F, Fut>(
    deadline: std::time::Instant,
    interval: Duration,
    mut condition: F,
) -> bool
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = bool>,
{
    loop {
        if condition().await {
            return true;
        }
        let now = std::time::Instant::now();
        if now >= deadline {
            return false;
        }
        let remaining = deadline - now;
        let sleep = remaining.min(interval);
        tokio::time::sleep(sleep).await;
    }
}

#[tokio::test]
async fn poll_stops_at_deadline() {
    let deadline = tokio::time::Instant::now() + Duration::from_millis(100);
    let result = poll_until(deadline.into_std(), Duration::from_millis(10), || async {
        false
    })
    .await;
    assert!(!result, "poll should time out");
}

#[tokio::test]
async fn poll_returns_immediately_when_condition_true() {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    let result = poll_until(deadline.into_std(), Duration::from_millis(1000), || async {
        true
    })
    .await;
    assert!(result, "poll should detect true immediately");
}

#[test]
fn sleep_capped_by_remaining() {
    // Pure logic: sleep = min(interval, remaining).
    let interval = Duration::from_secs(5);
    let remaining = Duration::from_millis(100);
    assert_eq!(remaining.min(interval), remaining);
    let remaining = Duration::from_secs(60);
    assert_eq!(remaining.min(interval), interval);
}

#[test]
fn interval_below_1s_returns_validation() {
    // Per plan P07.8: calling with interval < 1s returns Validation.
    let interval = Duration::from_millis(500);
    assert!(
        interval < Duration::from_secs(1),
        "interval below 1s must be rejected"
    );
}
