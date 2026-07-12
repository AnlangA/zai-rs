//! Cross-module behavior tests for the tool registry and executor.

#![cfg(feature = "toolkits")]

use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use serde_json::json;
use tokio::sync::{Notify, Semaphore};
use zai_rs::{
    model::chat_base_response::{ToolCallMessage, ToolFunction},
    toolkits::{core::FunctionTool, executor::ToolExecutor},
};

fn counting_echo(name: &str, calls: Arc<AtomicUsize>, label: &'static str) -> FunctionTool {
    FunctionTool::builder(name, "echo a value")
        .property("value", json!({"type": "integer"}))
        .handler(move |arguments| {
            let calls = Arc::clone(&calls);
            async move {
                calls.fetch_add(1, Ordering::SeqCst);
                Ok(json!({"source": label, "value": arguments["value"]}))
            }
        })
        .build()
        .unwrap()
}

#[tokio::test]
async fn late_old_result_cannot_pollute_re_registered_tool_cache() {
    let started = Arc::new(Notify::new());
    let release = Arc::new(Notify::new());
    let old_tool = FunctionTool::builder("replaceable", "old registration")
        .property("value", json!({"type": "integer"}))
        .handler({
            let started = Arc::clone(&started);
            let release = Arc::clone(&release);
            move |_| {
                let started = Arc::clone(&started);
                let release = Arc::clone(&release);
                async move {
                    started.notify_one();
                    release.notified().await;
                    Ok(json!({"source": "old"}))
                }
            }
        })
        .build()
        .unwrap();
    let executor = ToolExecutor::builder().enable_cache().build();
    executor.add_dyn_tool(Box::new(old_tool)).unwrap();

    let old_execution = {
        let executor = executor.clone();
        tokio::spawn(async move {
            executor
                .execute_simple("replaceable", json!({"value": 1}))
                .await
        })
    };
    started.notified().await;
    executor.unregister("replaceable").unwrap();

    let new_calls = Arc::new(AtomicUsize::new(0));
    executor
        .add_dyn_tool(Box::new(counting_echo(
            "replaceable",
            Arc::clone(&new_calls),
            "new",
        )))
        .unwrap();
    release.notify_one();
    assert_eq!(old_execution.await.unwrap().unwrap()["source"], "old");
    assert_eq!(executor.cache_stats().total_entries, 0);

    let new_result = executor
        .execute_simple("replaceable", json!({"value": 1}))
        .await
        .unwrap();
    assert_eq!(new_result["source"], "new");
    assert_eq!(new_calls.load(Ordering::SeqCst), 1);
}

#[test]
fn registry_exports_are_deterministic_and_filterable() {
    let executor = ToolExecutor::new();
    executor
        .add_dyn_tool(Box::new(counting_echo(
            "zeta",
            Arc::new(AtomicUsize::new(0)),
            "z",
        )))
        .unwrap();
    executor
        .add_dyn_tool(Box::new(counting_echo(
            "alpha",
            Arc::new(AtomicUsize::new(0)),
            "a",
        )))
        .unwrap();

    assert_eq!(executor.tool_names(), ["alpha", "zeta"]);
    let exported = executor.export_all_tools_as_functions();
    let names: Vec<_> = exported
        .iter()
        .filter_map(|tool| match tool {
            zai_rs::model::Tools::Function { function } => Some(function.name.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(names, ["alpha", "zeta"]);
    assert_eq!(
        executor
            .export_tools_filtered(|metadata| metadata.name() == "zeta")
            .len(),
        1
    );
}

#[test]
fn disabled_tools_are_never_advertised_to_the_model() {
    let executor = ToolExecutor::new();
    let disabled = FunctionTool::builder("disabled", "must not be called")
        .metadata(|metadata| metadata.with_enabled(false))
        .handler(|_| async { Ok(json!({"unexpected": true})) })
        .build()
        .unwrap();
    executor.add_dyn_tool(Box::new(disabled)).unwrap();

    assert!(executor.export_tool_as_function("disabled").is_none());
    assert!(executor.export_all_tools_as_functions().is_empty());
    assert!(executor.export_tools_filtered(|_| true).is_empty());
}

#[tokio::test]
async fn handler_panics_become_non_retryable_failures() {
    let executor = ToolExecutor::builder().retries(3).build();
    let tool = FunctionTool::builder("panics", "panic isolation fixture")
        .handler(|_| async { panic!("sensitive panic payload") })
        .build()
        .unwrap();
    executor.add_dyn_tool(Box::new(tool)).unwrap();

    let result = executor.execute("panics", json!({})).await.unwrap();

    assert!(!result.success);
    assert_eq!(result.retries, 0);
    let error = result.error.expect("panic must produce an error");
    assert!(error.contains("panicked"));
    assert!(!error.contains("sensitive panic payload"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn parallel_tool_calls_are_bounded_by_executor_limit() {
    let active = Arc::new(AtomicUsize::new(0));
    let peak = Arc::new(AtomicUsize::new(0));
    let all_slots_active = Arc::new(Notify::new());
    let release = Arc::new(Semaphore::new(0));
    let tool = FunctionTool::builder("bounded", "records concurrency")
        .handler({
            let active = Arc::clone(&active);
            let peak = Arc::clone(&peak);
            let all_slots_active = Arc::clone(&all_slots_active);
            let release = Arc::clone(&release);
            move |_| {
                let active = Arc::clone(&active);
                let peak = Arc::clone(&peak);
                let all_slots_active = Arc::clone(&all_slots_active);
                let release = Arc::clone(&release);
                async move {
                    let current = active.fetch_add(1, Ordering::SeqCst) + 1;
                    peak.fetch_max(current, Ordering::SeqCst);
                    if current == 8 {
                        all_slots_active.notify_one();
                    }
                    release.acquire().await.unwrap().forget();
                    active.fetch_sub(1, Ordering::SeqCst);
                    Ok(json!({"ok": true}))
                }
            }
        })
        .build()
        .unwrap();
    let executor = ToolExecutor::new();
    executor.add_dyn_tool(Box::new(tool)).unwrap();
    let calls: Vec<_> = (0..24)
        .map(|index| ToolCallMessage {
            id: Some(format!("call_{index}")),
            type_: Some("function".to_string()),
            function: Some(ToolFunction {
                name: "bounded".to_string(),
                arguments: "{}".to_string(),
            }),
            mcp: None,
        })
        .collect();

    let unblock_when_full = async {
        tokio::time::timeout(Duration::from_secs(1), all_slots_active.notified())
            .await
            .expect("executor never filled its documented eight-call parallelism limit");
        assert_eq!(peak.load(Ordering::SeqCst), 8);
        release.add_permits(calls.len());
    };
    let (results, ()) = tokio::join!(
        executor.execute_tool_calls_parallel(&calls),
        unblock_when_full,
    );

    assert_eq!(results.len(), calls.len());
    assert!(results.iter().all(|result| {
        let zai_rs::model::TextMessage::Tool { content, .. } = result else {
            return false;
        };
        serde_json::from_str::<serde_json::Value>(content)
            .is_ok_and(|value| value == json!({"ok": true}))
    }));
}
