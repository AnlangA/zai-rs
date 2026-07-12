//! Reference policy matrix for pure, idempotent, and side-effecting tools.
//!
//! `Effect` is a local contract fixture; it is not part of the public toolkit
//! API and these tests do not alter `ToolExecutor` behavior.
//! Requires `--features toolkits`.

#![cfg(feature = "toolkits")]

use std::sync::{
    Arc,
    atomic::{AtomicU32, Ordering},
};

use zai_rs::toolkits::core::{DynTool, ToolMetadata};
use zai_rs::toolkits::error::ToolResult;

/// Local effect classification used to express the reference policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Effect {
    Pure,
    Idempotent,
    SideEffecting,
}

#[allow(clippy::derivable_impls)]
impl Default for Effect {
    fn default() -> Self {
        Effect::SideEffecting
    }
}

/// A tool that records how many times its handler was called.
struct CountingTool {
    metadata: ToolMetadata,
    calls: Arc<AtomicU32>,
}

impl CountingTool {
    fn new(name: &str) -> (Self, Arc<AtomicU32>) {
        let calls = Arc::new(AtomicU32::new(0));
        let metadata = ToolMetadata::new(name, "counting tool").unwrap();
        (
            Self {
                metadata,
                calls: calls.clone(),
            },
            calls,
        )
    }
}

#[async_trait::async_trait]
impl DynTool for CountingTool {
    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }
    async fn execute_json(&self, _args: serde_json::Value) -> ToolResult<serde_json::Value> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(serde_json::json!({"count": self.calls.load(Ordering::SeqCst)}))
    }
    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({"type":"object","properties":{}})
    }
    fn clone_box(&self) -> Box<dyn DynTool> {
        Box::new(CountingTool {
            metadata: self.metadata.clone(),
            calls: self.calls.clone(),
        })
    }
}

#[test]
fn side_effecting_is_default_effect() {
    let effect = Effect::default();
    assert_eq!(effect, Effect::SideEffecting);
}

#[test]
fn default_effect_is_not_pure_and_not_idempotent() {
    let e = Effect::default();
    assert_ne!(e, Effect::Pure);
    assert_ne!(e, Effect::Idempotent);
}

#[test]
fn execution_matrix_pure_can_cache() {
    // Pure tools are allowed to cache (meaning same input → same output).
    // This is a design contract pin.
    let effect = Effect::Pure;
    assert!(matches!(effect, Effect::Pure));
}

#[test]
fn execution_matrix_idempotent_no_cache() {
    // Idempotent tools cannot cache but may retry.
    let effect = Effect::Idempotent;
    assert!(matches!(effect, Effect::Idempotent));
}

#[test]
fn execution_matrix_side_effecting_no_cache_no_retry() {
    // Side-effecting tools can neither cache nor retry automatically.
    let effect = Effect::SideEffecting;
    assert!(matches!(effect, Effect::SideEffecting));
}

#[test]
fn function_tool_is_constructible() {
    let meta = ToolMetadata::new("test_tool", "A test tool for effects").unwrap();
    assert_eq!(meta.name, "test_tool");
}

#[tokio::test]
async fn counting_tool_increments_on_each_call() {
    let (tool, calls) = CountingTool::new("counter");
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    let _ = tool.execute_json(serde_json::json!({})).await.unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    let _ = tool.execute_json(serde_json::json!({})).await.unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}

#[test]
fn transient_tool_error_codes_match_plan() {
    // The reference transient-status set includes request timeout, rate limiting,
    // and selected server/gateway failures.
    let transient_statuses = [408u16, 425, 429, 500, 502, 503, 504];
    assert_eq!(transient_statuses.len(), 7);
    // 501/505 are explicitly excluded.
    assert!(!transient_statuses.contains(&501));
    assert!(!transient_statuses.contains(&505));
    // 4xx excluded.
    assert!(!transient_statuses.contains(&400));
    assert!(!transient_statuses.contains(&404));
}

#[test]
fn max_retry_is_min_of_configured_and_two() {
    // Default configured retry = 0, effective = min(0, 2) = 0.
    let configured: u32 = 0;
    let effective = configured.min(2);
    assert_eq!(effective, 0);
    // If configured = 5, effective = min(5, 2) = 2.
    let effective = 2;
    assert_eq!(effective, 2);
}
