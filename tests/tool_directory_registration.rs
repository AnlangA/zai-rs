//! Behavioral contracts for trusted directory tool registration.

#![cfg(feature = "toolkits")]

use std::{
    collections::HashMap,
    future::Future,
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use serde_json::{Value, json};
use tokio::sync::Barrier;
use zai_rs::toolkits::{
    CachePolicy, FunctionTool, RetryPolicy, ToolExecutionPolicy, ToolHandler, ToolRegistration,
    error::{ToolResult, error_context},
    executor::ToolExecutor,
};

fn handler<F, Fut>(function: F) -> ToolHandler
where
    F: Fn(Value) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = ToolResult<Value>> + Send + 'static,
{
    Arc::new(
        move |arguments| -> Pin<Box<dyn Future<Output = ToolResult<Value>> + Send>> {
            Box::pin(function(arguments))
        },
    )
}

fn write_spec(directory: &std::path::Path, file_name: &str, spec: Value) {
    std::fs::write(
        directory.join(file_name),
        serde_json::to_vec_pretty(&spec).unwrap(),
    )
    .unwrap();
}

fn direct_spec(name: &str) -> Value {
    json!({
        "name": name,
        "description": format!("{name} fixture"),
        "parameters": {
            "type": "object",
            "additionalProperties": true
        }
    })
}

#[tokio::test]
async fn legacy_registry_stays_never_cache_never_retry() {
    let directory = tempfile::tempdir().unwrap();
    write_spec(directory.path(), "legacy.json", direct_spec("legacy"));

    let calls = Arc::new(AtomicUsize::new(0));
    let mut handlers = HashMap::new();
    handlers.insert(
        "legacy".to_string(),
        handler({
            let calls = Arc::clone(&calls);
            move |_| {
                let calls = Arc::clone(&calls);
                async move {
                    let attempt = calls.fetch_add(1, Ordering::SeqCst) + 1;
                    if attempt == 1 {
                        Err(error_context()
                            .with_tool("legacy")
                            .execution_failed("transient fixture"))
                    } else {
                        Ok(json!({"attempt": attempt}))
                    }
                }
            }
        }),
    );

    let executor = ToolExecutor::builder().enable_cache().retries(2).build();
    assert_eq!(
        executor
            .add_functions_from_dir_with_registry(directory.path(), &handlers, true)
            .unwrap(),
        vec!["legacy"]
    );

    let first = executor.execute("legacy", json!({})).await.unwrap();
    assert!(!first.success);
    assert_eq!(first.retries, 0);
    assert_eq!(calls.load(Ordering::SeqCst), 1);

    let second = executor.execute("legacy", json!({})).await.unwrap();
    let third = executor.execute("legacy", json!({})).await.unwrap();
    assert_eq!(second.result, json!({"attempt": 2}));
    assert_eq!(third.result, json!({"attempt": 3}));
    assert!(!second.cache_hit);
    assert!(!third.cache_hit);
    assert_eq!(calls.load(Ordering::SeqCst), 3);
    assert_eq!(executor.cache_stats().total_entries, 0);
}

#[tokio::test]
async fn pure_registration_uses_cache_singleflight() {
    const CALLERS: usize = 12;

    let directory = tempfile::tempdir().unwrap();
    write_spec(directory.path(), "pure.json", direct_spec("pure_directory"));

    let calls = Arc::new(AtomicUsize::new(0));
    let registration = ToolRegistration::new(handler({
        let calls = Arc::clone(&calls);
        move |_| {
            let calls = Arc::clone(&calls);
            async move {
                calls.fetch_add(1, Ordering::SeqCst);
                tokio::time::sleep(Duration::from_millis(25)).await;
                Ok(json!({"shared": true}))
            }
        }
    }))
    .with_execution_policy(ToolExecutionPolicy::new(
        CachePolicy::Pure,
        RetryPolicy::Never,
    ));
    assert_eq!(
        registration.execution_policy().cache_policy(),
        CachePolicy::Pure
    );
    let registrations = HashMap::from([("pure_directory".to_string(), registration)]);

    let executor = ToolExecutor::builder().enable_cache().build();
    executor
        .add_functions_from_dir_with_registrations(directory.path(), &registrations, true)
        .unwrap();
    let barrier = Arc::new(Barrier::new(CALLERS));
    let tasks = (0..CALLERS)
        .map(|_| {
            let executor = executor.clone();
            let barrier = Arc::clone(&barrier);
            tokio::spawn(async move {
                barrier.wait().await;
                executor.execute("pure_directory", json!({})).await.unwrap()
            })
        })
        .collect::<Vec<_>>();

    let mut cache_hits = 0;
    for task in tasks {
        let result = task.await.unwrap();
        assert!(result.success);
        assert_eq!(result.result, json!({"shared": true}));
        cache_hits += usize::from(result.cache_hit);
    }

    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(cache_hits, CALLERS - 1);
}

#[tokio::test]
async fn json_policy_fields_cannot_elevate_a_safe_registration() {
    let directory = tempfile::tempdir().unwrap();
    write_spec(
        directory.path(),
        "forged.json",
        json!({
            "type": "function",
            "execution_policy": {
                "cache": "pure",
                "retry": "idempotent"
            },
            "function": {
                "name": "forged",
                "description": "untrusted policy fixture",
                "parameters": {
                    "type": "object",
                    "additionalProperties": true
                },
                "execution_policy": {
                    "cache": "pure",
                    "retry": "idempotent"
                }
            }
        }),
    );

    let calls = Arc::new(AtomicUsize::new(0));
    let registration = ToolRegistration::new(handler({
        let calls = Arc::clone(&calls);
        move |_| {
            let calls = Arc::clone(&calls);
            async move {
                let attempt = calls.fetch_add(1, Ordering::SeqCst) + 1;
                if attempt == 1 {
                    Err(error_context()
                        .with_tool("forged")
                        .execution_failed("transient fixture"))
                } else {
                    Ok(json!({"attempt": attempt}))
                }
            }
        }
    }));
    assert_eq!(
        registration.execution_policy(),
        ToolExecutionPolicy::default()
    );
    let registrations = HashMap::from([("forged".to_string(), registration)]);
    let executor = ToolExecutor::builder().enable_cache().retries(2).build();
    executor
        .add_functions_from_dir_with_registrations(directory.path(), &registrations, true)
        .unwrap();

    let first = executor.execute("forged", json!({})).await.unwrap();
    assert!(!first.success);
    assert_eq!(first.retries, 0);
    let second = executor.execute("forged", json!({})).await.unwrap();
    let third = executor.execute("forged", json!({})).await.unwrap();

    assert_eq!(second.result, json!({"attempt": 2}));
    assert_eq!(third.result, json!({"attempt": 3}));
    assert!(!second.cache_hit);
    assert!(!third.cache_hit);
    assert_eq!(calls.load(Ordering::SeqCst), 3);
    assert_eq!(executor.cache_stats().total_entries, 0);
}

#[tokio::test]
async fn strict_missing_registration_is_all_or_nothing() {
    let directory = tempfile::tempdir().unwrap();
    write_spec(directory.path(), "01_known.json", direct_spec("known"));
    write_spec(directory.path(), "02_missing.json", direct_spec("missing"));

    let echo = handler(|arguments| async move { Ok(arguments) });
    let registrations = HashMap::from([
        (
            "known".to_string(),
            ToolRegistration::new(Arc::clone(&echo)),
        ),
        (
            "unused".to_string(),
            ToolRegistration::new(Arc::clone(&echo)),
        ),
    ]);
    let executor = ToolExecutor::new();

    let error = executor
        .add_functions_from_dir_with_registrations(directory.path(), &registrations, true)
        .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("No handler registered for function 'missing'")
    );
    assert!(!executor.has_tool("known"));
    assert!(!executor.has_tool("missing"));

    assert_eq!(
        executor
            .add_functions_from_dir_with_registrations(directory.path(), &registrations, false,)
            .unwrap(),
        vec!["known"]
    );
    assert!(executor.has_tool("known"));
    assert!(!executor.has_tool("missing"));
    assert!(!executor.has_tool("unused"));
}

#[test]
fn directory_conflict_and_duplicate_name_reject_the_whole_batch() {
    let echo = handler(|arguments| async move { Ok(arguments) });

    let conflict_directory = tempfile::tempdir().unwrap();
    write_spec(
        conflict_directory.path(),
        "01_fresh.json",
        direct_spec("fresh"),
    );
    write_spec(
        conflict_directory.path(),
        "02_existing.json",
        direct_spec("existing"),
    );
    let registrations = HashMap::from([
        (
            "fresh".to_string(),
            ToolRegistration::new(Arc::clone(&echo)),
        ),
        (
            "existing".to_string(),
            ToolRegistration::new(Arc::clone(&echo)),
        ),
    ]);
    let executor = ToolExecutor::new();
    let existing = FunctionTool::builder("existing", "existing fixture")
        .handler(|arguments| async move { Ok(arguments) })
        .build()
        .unwrap();
    executor.add_dyn_tool(Box::new(existing)).unwrap();

    let error = executor
        .add_functions_from_dir_with_registrations(conflict_directory.path(), &registrations, true)
        .unwrap_err();
    assert!(error.to_string().contains("already registered"));
    assert!(!executor.has_tool("fresh"));
    assert!(executor.has_tool("existing"));

    let duplicate_directory = tempfile::tempdir().unwrap();
    write_spec(
        duplicate_directory.path(),
        "01_duplicate.json",
        direct_spec("duplicate"),
    );
    write_spec(
        duplicate_directory.path(),
        "02_duplicate.json",
        direct_spec("duplicate"),
    );
    let duplicate_registrations =
        HashMap::from([("duplicate".to_string(), ToolRegistration::new(echo))]);
    let duplicate_executor = ToolExecutor::new();

    let error = duplicate_executor
        .add_functions_from_dir_with_registrations(
            duplicate_directory.path(),
            &duplicate_registrations,
            true,
        )
        .unwrap_err();
    assert!(error.to_string().contains("Duplicate function name"));
    assert!(!duplicate_executor.has_tool("duplicate"));
}
