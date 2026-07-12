//! Behavioral contracts for cache and retry opt-in semantics.

#![cfg(feature = "toolkits")]

use std::sync::{
    Arc,
    atomic::{AtomicU32, Ordering},
};
use std::time::Duration;

use serde_json::json;
use zai_rs::toolkits::{core::FunctionTool, error::error_context, executor::ToolExecutor};

#[tokio::test]
async fn default_executor_does_not_cache_unknown_tool_effects() {
    let calls = Arc::new(AtomicU32::new(0));
    let handler_calls = Arc::clone(&calls);
    let tool = FunctionTool::builder("counter", "count executions")
        .handler(move |_| {
            let calls = Arc::clone(&handler_calls);
            async move { Ok(json!({"call": calls.fetch_add(1, Ordering::SeqCst) + 1})) }
        })
        .build()
        .unwrap();
    let executor = ToolExecutor::new();
    executor.add_dyn_tool(Box::new(tool)).unwrap();

    executor.execute_simple("counter", json!({})).await.unwrap();
    executor.execute_simple("counter", json!({})).await.unwrap();

    assert_eq!(calls.load(Ordering::SeqCst), 2);
    assert_eq!(executor.cache_stats().total_entries, 0);
}

#[tokio::test]
async fn cache_options_do_not_enable_caching_implicitly() {
    let calls = Arc::new(AtomicU32::new(0));
    let handler_calls = Arc::clone(&calls);
    let tool = FunctionTool::builder("configured", "cache configuration fixture")
        .handler(move |_| {
            let calls = Arc::clone(&handler_calls);
            async move { Ok(json!({"call": calls.fetch_add(1, Ordering::SeqCst) + 1})) }
        })
        .build()
        .unwrap();
    let executor = ToolExecutor::builder()
        .cache_ttl(Duration::from_secs(60))
        .cache_max_size(8)
        .build();
    executor.add_dyn_tool(Box::new(tool)).unwrap();

    executor
        .execute_simple("configured", json!({}))
        .await
        .unwrap();
    executor
        .execute_simple("configured", json!({}))
        .await
        .unwrap();

    assert_eq!(calls.load(Ordering::SeqCst), 2);
    assert_eq!(executor.cache_stats().total_entries, 0);
}

#[tokio::test]
async fn default_executor_does_not_retry_unknown_tool_effects() {
    let calls = Arc::new(AtomicU32::new(0));
    let handler_calls = Arc::clone(&calls);
    let tool = FunctionTool::builder("side_effect", "always fails")
        .handler(move |_| {
            let calls = Arc::clone(&handler_calls);
            async move {
                calls.fetch_add(1, Ordering::SeqCst);
                Err(error_context()
                    .with_tool("side_effect")
                    .execution_failed("failed after side effect"))
            }
        })
        .build()
        .unwrap();
    let executor = ToolExecutor::new();
    executor.add_dyn_tool(Box::new(tool)).unwrap();

    let result = executor.execute("side_effect", json!({})).await.unwrap();

    assert!(!result.success);
    assert_eq!(result.retries, 0);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn pure_tool_can_opt_in_to_caching() {
    let calls = Arc::new(AtomicU32::new(0));
    let handler_calls = Arc::clone(&calls);
    let tool = FunctionTool::builder("pure", "deterministic tool")
        .property("n", json!({"type": "integer"}))
        .handler(move |arguments| {
            let calls = Arc::clone(&handler_calls);
            async move {
                calls.fetch_add(1, Ordering::SeqCst);
                Ok(arguments)
            }
        })
        .build()
        .unwrap();
    let executor = ToolExecutor::builder().enable_cache().build();
    executor.add_dyn_tool(Box::new(tool)).unwrap();

    executor.execute("pure", json!({"n": 1})).await.unwrap();
    let cached = executor.execute("pure", json!({"n": 1})).await.unwrap();

    assert!(cached.success);
    assert!(cached.cache_hit);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn idempotent_tool_can_opt_in_to_retries() {
    let calls = Arc::new(AtomicU32::new(0));
    let handler_calls = Arc::clone(&calls);
    let tool = FunctionTool::builder("flaky", "fails twice")
        .handler(move |_| {
            let calls = Arc::clone(&handler_calls);
            async move {
                let attempt = calls.fetch_add(1, Ordering::SeqCst) + 1;
                if attempt < 3 {
                    Err(error_context()
                        .with_tool("flaky")
                        .execution_failed("temporary failure"))
                } else {
                    Ok(json!({"attempt": attempt}))
                }
            }
        })
        .build()
        .unwrap();
    let executor = ToolExecutor::builder().retries(2).build();
    executor.add_dyn_tool(Box::new(tool)).unwrap();

    let result = executor.execute("flaky", json!({})).await.unwrap();

    assert!(result.success);
    assert_eq!(result.retries, 2);
    assert_eq!(calls.load(Ordering::SeqCst), 3);
}
