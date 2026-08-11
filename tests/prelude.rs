//! Compile-time smoke coverage for the intentionally small common prelude.

use std::time::Duration;

use zai_rs::prelude::*;

#[test]
fn prelude_exposes_client_and_scoped_transport_policy() {
    let transport = HttpTransportConfig::default().with_max_attempts(2).unwrap();
    let concurrency = HttpConcurrencyConfig::default()
        .with_max_in_flight(7)
        .unwrap()
        .with_queue_timeout(Duration::from_secs(3))
        .unwrap()
        .with_stream_consumer_timeout(Duration::from_secs(4 * 60))
        .unwrap();
    let concurrency_clone = concurrency.clone();
    let options = RequestOptions::default()
        .with_queue_timeout(Duration::from_millis(250))
        .unwrap()
        .with_attempt_timeout(Duration::from_secs(5))
        .unwrap()
        .with_stream_consumer_timeout(Duration::from_secs(2 * 60))
        .unwrap()
        .with_retry_override(RetryOverride::AssumeIdempotent);
    let client = ZaiClient::builder("test.12345678901234567890")
        .endpoint(ApiFamily::PaasV4, "https://open.bigmodel.cn/api/paas/v4")
        .transport(transport)
        .concurrency(concurrency)
        .build()
        .unwrap()
        .with_request_options(options);
    let cloned = client.clone();

    assert_eq!(client.transport().max_attempts, 2);
    assert_eq!(client.concurrency().max_in_flight(), 7);
    assert_eq!(client.concurrency().queue_timeout(), Duration::from_secs(3));
    // Public getters expose the configured base values, not the effective
    // `max(base, sse_idle + 1s)` interval used by an SSE stream.
    assert_eq!(
        client.concurrency().stream_consumer_timeout(),
        Duration::from_secs(4 * 60)
    );
    assert_eq!(client.concurrency(), &concurrency_clone);
    assert!(std::ptr::eq(client.concurrency(), cloned.concurrency()));
    assert_eq!(
        client.request_options().queue_timeout(),
        Some(Duration::from_millis(250))
    );
    assert_eq!(
        client.request_options().stream_consumer_timeout(),
        Some(Duration::from_secs(2 * 60))
    );
    assert_eq!(client.request_options().max_attempts(), None);
    assert_eq!(
        client.request_options().retry_override(),
        Some(RetryOverride::AssumeIdempotent)
    );

    let concurrency_debug = format!("{concurrency_clone:?}");
    let options_debug = format!("{options:?}");
    let client_debug = format!("{cloned:?}");
    assert!(concurrency_debug.contains("stream_consumer_timeout"));
    assert!(options_debug.contains("stream_consumer_timeout"));
    assert!(client_debug.contains("[REDACTED]"));
    assert!(!client_debug.contains("12345678901234567890"));
}
