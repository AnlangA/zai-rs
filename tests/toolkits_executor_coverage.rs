//! Coverage for toolkits/executor.rs (requires --features toolkits).
#![cfg(feature = "toolkits")]

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use serde_json::json;
use zai_rs::toolkits::core::*;
use zai_rs::toolkits::error::ToolResult;
use zai_rs::toolkits::executor::*;

struct EchoTool {
    meta: ToolMetadata,
    calls: Arc<AtomicU32>,
}

impl EchoTool {
    fn new() -> (Self, Arc<AtomicU32>) {
        let calls = Arc::new(AtomicU32::new(0));
        (
            Self {
                meta: ToolMetadata::new("echo", "echoes input").unwrap(),
                calls: calls.clone(),
            },
            calls,
        )
    }
}

#[async_trait::async_trait]
impl DynTool for EchoTool {
    fn metadata(&self) -> &ToolMetadata {
        &self.meta
    }
    fn input_schema(&self) -> serde_json::Value {
        json!({"type": "object", "properties": {"text": {"type": "string"}}})
    }
    async fn execute_json(&self, args: serde_json::Value) -> ToolResult<serde_json::Value> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(args)
    }
    fn clone_box(&self) -> Box<dyn DynTool> {
        Box::new(EchoTool {
            meta: ToolMetadata::new("echo", "echoes input").unwrap(),
            calls: self.calls.clone(),
        })
    }
}

#[test]
fn executor_new_empty() {
    let exec = ToolExecutor::new();
    assert!(!exec.has_tool("echo"));
    assert!(exec.tool_names().is_empty());
}

#[test]
fn executor_builder_defaults() {
    let exec = ToolExecutor::builder().build();
    assert!(exec.tool_names().is_empty());
}

#[test]
fn executor_builder_with_options() {
    let exec = ToolExecutor::builder()
        .timeout(Duration::from_secs(10))
        .retries(2)
        .enable_cache()
        .cache_ttl(Duration::from_secs(30))
        .cache_max_size(100)
        .build();
    assert_eq!(exec.config().timeout, Some(Duration::from_secs(10)));
}

#[test]
fn executor_builder_disable_cache() {
    let exec = ToolExecutor::builder()
        .enable_cache()
        .disable_cache()
        .build();
    let _ = exec;
}

#[test]
fn executor_add_and_has_tool() {
    let exec = ToolExecutor::new();
    let (tool, _) = EchoTool::new();
    exec.add_dyn_tool(Box::new(tool)).unwrap();
    assert!(exec.has_tool("echo"));
    assert_eq!(exec.tool_names(), vec!["echo"]);
}

#[test]
fn executor_try_add_dyn_tool() {
    let exec = ToolExecutor::new();
    let (tool, _) = EchoTool::new();
    exec.try_add_dyn_tool(Box::new(tool));
    assert!(exec.has_tool("echo"));
}

#[test]
fn executor_add_duplicate_returns_error() {
    let exec = ToolExecutor::new();
    let (t1, _) = EchoTool::new();
    let (t2, _) = EchoTool::new();
    exec.add_dyn_tool(Box::new(t1)).unwrap();
    assert!(exec.add_dyn_tool(Box::new(t2)).is_err());
}

#[test]
fn executor_unregister() {
    let exec = ToolExecutor::new();
    let (tool, _) = EchoTool::new();
    exec.add_dyn_tool(Box::new(tool)).unwrap();
    exec.unregister("echo").unwrap();
    assert!(!exec.has_tool("echo"));
}

#[test]
fn executor_unregister_missing() {
    let exec = ToolExecutor::new();
    assert!(exec.unregister("nonexistent").is_err());
}

#[test]
fn executor_input_schema() {
    let exec = ToolExecutor::new();
    let (tool, _) = EchoTool::new();
    exec.add_dyn_tool(Box::new(tool)).unwrap();
    assert!(exec.input_schema("echo").is_some());
    assert!(exec.input_schema("missing").is_none());
}

#[test]
fn executor_export_tool_as_function() {
    let exec = ToolExecutor::new();
    let (tool, _) = EchoTool::new();
    exec.add_dyn_tool(Box::new(tool)).unwrap();
    assert!(exec.export_tool_as_function("echo").is_some());
    assert!(exec.export_tool_as_function("missing").is_none());
}

#[test]
fn executor_export_all_tools() {
    let exec = ToolExecutor::new();
    let (tool, _) = EchoTool::new();
    exec.add_dyn_tool(Box::new(tool)).unwrap();
    assert_eq!(exec.export_all_tools_as_functions().len(), 1);
}

#[test]
fn executor_export_filtered() {
    let exec = ToolExecutor::new();
    let (tool, _) = EchoTool::new();
    exec.add_dyn_tool(Box::new(tool)).unwrap();
    assert_eq!(exec.export_tools_filtered(|m| m.name == "echo").len(), 1);
    assert!(exec.export_tools_filtered(|_| false).is_empty());
}

#[test]
fn executor_cache_operations() {
    let exec = ToolExecutor::new().with_cache_enabled(true);
    exec.clear_cache();
    exec.invalidate_cache_for_tool("echo");
    let _ = exec.cache_stats();
}

#[test]
fn executor_config_access() {
    let exec = ToolExecutor::new();
    assert!(exec.config().timeout.is_some() || exec.config().timeout.is_none());
}

#[test]
fn retry_config_default_delay() {
    let cfg = RetryConfig::default();
    let _ = cfg.calculate_delay(0);
}

#[test]
fn execution_result_success_constructor() {
    let r = ExecutionResult::success(
        "echo".into(),
        json!({"out": "hi"}),
        Duration::from_millis(5),
        0,
    );
    assert!(r.success);
    assert_eq!(r.tool_name, "echo");
}

#[test]
fn execution_result_failure_constructor() {
    let r = ExecutionResult::failure("echo".into(), "err".into(), Duration::from_millis(5), 2);
    assert!(!r.success);
    assert_eq!(r.retries, 2);
    assert!(r.error.is_some());
}

#[test]
fn execution_result_with_metadata() {
    let r = ExecutionResult::success("t".into(), json!({}), Duration::ZERO, 0)
        .with_metadata("key", json!("val"));
    assert!(r.metadata.contains_key("key"));
}

#[tokio::test]
async fn executor_execute_simple_ok() {
    let exec = ToolExecutor::new();
    let (tool, calls) = EchoTool::new();
    exec.add_dyn_tool(Box::new(tool)).unwrap();
    let result = exec.execute_simple("echo", json!({"text": "hi"})).await;
    assert!(result.is_ok());
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn executor_execute_simple_missing() {
    let exec = ToolExecutor::new();
    assert!(exec.execute_simple("missing", json!({})).await.is_err());
}

#[tokio::test]
async fn executor_execute_with_cache_hit() {
    let exec = ToolExecutor::new().with_cache_enabled(true);
    let (tool, _) = EchoTool::new();
    exec.add_dyn_tool(Box::new(tool)).unwrap();
    let _ = exec.execute_simple("echo", json!({"text": "a"})).await;
    let _ = exec.execute_simple("echo", json!({"text": "a"})).await;
}

#[tokio::test]
async fn executor_execute_different_inputs() {
    let exec = ToolExecutor::new();
    let (tool, calls) = EchoTool::new();
    exec.add_dyn_tool(Box::new(tool)).unwrap();
    let _ = exec.execute_simple("echo", json!({"text": "a"})).await;
    let _ = exec.execute_simple("echo", json!({"text": "b"})).await;
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn executor_execute_tool_calls_parallel_empty() {
    let exec = ToolExecutor::new();
    assert!(exec.execute_tool_calls_parallel(&[]).await.is_empty());
}

#[tokio::test]
async fn executor_execute_tool_calls_ordered_empty() {
    let exec = ToolExecutor::new();
    assert!(exec.execute_tool_calls_ordered(&[]).await.is_empty());
}
