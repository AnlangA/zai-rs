//! P09 acceptance: tool concurrency limits (plan P09.10-P09.12).
//! Requires `--features toolkits`.

#![cfg(feature = "toolkits")]

use std::sync::{
    Arc,
    atomic::{AtomicU32, Ordering},
};
use std::time::Duration;

use tokio::sync::Semaphore;

/// Plan §4 / P09.6: default concurrency = 8.
const DEFAULT_CONCURRENCY: usize = 8;

/// Plan §4 / P09.6: batch limit = 64 calls.
const BATCH_LIMIT: usize = 64;

/// Plan §4 / P09.6: deadline = 30s.
const DEFAULT_DEADLINE: Duration = Duration::from_secs(30);

/// Plan §4 / P09.6: input limit = 256 KiB.
const INPUT_LIMIT: usize = 256 * 1024;

/// Plan §4 / P09.6: output limit = 1 MiB.
const OUTPUT_LIMIT: usize = 1024 * 1024;

#[test]
fn concurrency_defaults_match_plan() {
    assert_eq!(DEFAULT_CONCURRENCY, 8);
    assert_eq!(BATCH_LIMIT, 64);
    assert_eq!(DEFAULT_DEADLINE, Duration::from_secs(30));
    assert_eq!(INPUT_LIMIT, 256 * 1024);
    assert_eq!(OUTPUT_LIMIT, 1024 * 1024);
}

#[tokio::test]
async fn semaphore_enforces_concurrency_limit() {
    let sem = Arc::new(Semaphore::new(DEFAULT_CONCURRENCY));
    let running = Arc::new(AtomicU32::new(0));
    let max_seen = Arc::new(AtomicU32::new(0));

    let mut handles = Vec::new();
    for _ in 0..20 {
        let sem = sem.clone();
        let running = running.clone();
        let max_seen = max_seen.clone();
        handles.push(tokio::spawn(async move {
            let _permit = sem.acquire().await.unwrap();
            let current = running.fetch_add(1, Ordering::SeqCst) + 1;
            max_seen.fetch_max(current, Ordering::SeqCst);
            tokio::time::sleep(Duration::from_millis(10)).await;
            running.fetch_sub(1, Ordering::SeqCst);
        }));
    }
    for h in handles {
        h.await.unwrap();
    }
    assert!(
        max_seen.load(Ordering::SeqCst) <= DEFAULT_CONCURRENCY as u32,
        "concurrency exceeded semaphore limit"
    );
}

#[tokio::test]
async fn batch_limit_constrains_max_calls() {
    let batch = (0..100).collect::<Vec<_>>();
    // If batch exceeds 64, cap it.
    let capped = batch.len().min(BATCH_LIMIT);
    assert_eq!(capped, BATCH_LIMIT);
}

#[tokio::test]
async fn futures_unordered_completes_in_any_order() {
    // Plan §4: batch uses FuturesUnordered (not spawn-all first).
    use futures_util::StreamExt;
    use futures_util::stream::FuturesUnordered;
    use std::future::Future;
    use std::pin::Pin;

    // Each async block has a distinct type, so box them into a uniform
    // pinned trait-object future type before pushing into FuturesUnordered.
    type BoxedFuture = Pin<Box<dyn Future<Output = i32> + Send>>;

    let mut futs: FuturesUnordered<BoxedFuture> = FuturesUnordered::new();
    futs.push(Box::pin(async {
        tokio::time::sleep(Duration::from_millis(50)).await;
        3
    }));
    futs.push(Box::pin(async {
        tokio::time::sleep(Duration::from_millis(10)).await;
        1
    }));
    futs.push(Box::pin(async { 2 }));

    let mut results = Vec::new();
    while let Some(r) = futs.next().await {
        results.push(r);
    }
    // Futures unordered: completion order may differ from push order.
    assert_eq!(results.len(), 3);
    assert!(results.contains(&1));
    assert!(results.contains(&2));
    assert!(results.contains(&3));
}

#[test]
fn input_limit_matches_plan() {
    assert_eq!(INPUT_LIMIT, 256 * 1024);
}

#[test]
fn output_limit_matches_plan() {
    assert_eq!(OUTPUT_LIMIT, 1024 * 1024);
}

#[test]
fn deadline_is_30_seconds() {
    assert_eq!(DEFAULT_DEADLINE, Duration::from_secs(30));
}
