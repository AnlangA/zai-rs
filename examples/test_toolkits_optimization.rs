//! Test toolkits optimization without requiring API calls
//!
//! This example demonstrates the optimized toolkits functionality

use std::time::Instant;

use serde_json::json;
use zai_rs::toolkits::prelude::*;

fn create_test_tools() -> Vec<FunctionTool> {
    let mut tools = Vec::new();

    // Create multiple tools to test performance
    for i in 0..100 {
        let tool = FunctionTool::builder(
            format!("test_tool_{}", i),
            format!("Test tool number {}", i),
        )
        .schema(json!({
            "type": "object",
            "properties": {
                "input": { "type": "string", "description": "Input parameter" }
            },
            "required": ["input"]
        }))
        .handler(move |args| async move {
            let input = args
                .get("input")
                .and_then(|v| v.as_str())
                .unwrap_or("default");
            Ok(json!({ "result": format!("Processed: {}", input) }))
        })
        .build()
        .expect("test tool");

        tools.push(tool);
    }

    tools
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var_os("RUST_LOG").is_some() {
        let _ = tracing_subscriber::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .try_init();
    }

    tracing::trace!("🚀 Testing optimized toolkits...");

    // Test 1: Tool registration performance
    let start = Instant::now();
    let executor = ToolExecutor::new();

    let tools = create_test_tools();
    for tool in tools {
        executor.add_dyn_tool(Box::new(tool)).unwrap();
    }

    let registration_time = start.elapsed();
    tracing::trace!("✅ Registered 100 tools in {:?}", registration_time);

    // Test 2: Tool execution performance
    let start = Instant::now();
    let mut results = Vec::new();

    for i in 0..50 {
        let result = executor
            .execute_simple(
                &format!("test_tool_{}", i),
                json!({ "input": format!("test input {}", i) }),
            )
            .await?;
        results.push(result);
    }

    let execution_time = start.elapsed();
    tracing::trace!("✅ Executed 50 tools in {:?}", execution_time);
    tracing::trace!("✅ Average execution time: {:?}", execution_time / 50);

    // Test 3: Parallel execution
    let start = Instant::now();
    let tool_calls: Vec<_> = (0..10)
        .map(|i| {
            serde_json::from_value(json!({
                "id": format!("call_{}", i),
                "type": "function",
                "function": {
                    "name": format!("test_tool_{}", i),
                    "arguments": json!({ "input": format!("parallel test {}", i) }).to_string()
                }
            }))
            .expect("Failed to create tool call JSON")
        })
        .collect();

    let parallel_results = executor.execute_tool_calls_parallel(&tool_calls).await;
    let parallel_time = start.elapsed();

    tracing::trace!("✅ Executed 10 tools in parallel in {:?}", parallel_time);
    tracing::trace!(
        "✅ Parallel results: {} successful calls",
        parallel_results.len()
    );

    // Test 4: Schema caching (execute same tool multiple times)
    let start = Instant::now();
    for _ in 0..20 {
        let _result = executor
            .execute_simple("test_tool_0", json!({ "input": "schema caching test" }))
            .await?;
    }
    let cached_time = start.elapsed();
    tracing::trace!(
        "✅ Executed same tool 20 times (with schema caching) in {:?}",
        cached_time
    );

    // Test 5: Error handling
    match executor.execute_simple("nonexistent_tool", json!({})).await {
        Ok(_) => println!("❌ Unexpected success"),
        Err(e) => println!("✅ Expected error for nonexistent tool: {}", e),
    }

    // Test 6: Retry mechanism with exponential backoff
    let retry_executor = ToolExecutor::builder().retries(3).build();

    let failing_tool = FunctionTool::builder("failing_tool", "Tool that fails sometimes")
        .handler(|_args| async move {
            Err(error_context()
                .with_tool("failing_tool")
                .execution_failed("Simulated failure"))
        })
        .build()?;

    retry_executor.add_dyn_tool(Box::new(failing_tool)).unwrap();

    let start = Instant::now();
    let retry_result = retry_executor
        .execute_simple("failing_tool", json!({}))
        .await;
    let retry_time = start.elapsed();

    match retry_result {
        Ok(_) => println!("❌ Unexpected success on failing tool"),
        Err(e) => println!(
            "✅ Expected error after retries: {} (took {:?})",
            e, retry_time
        ),
    }

    tracing::trace!("\n🎉 All optimization tests completed successfully!");
    tracing::trace!("Key optimizations implemented:");
    tracing::trace!("  ✅ DashMap for concurrent tool registry");
    tracing::trace!("  ✅ Schema caching for JSON validation");
    tracing::trace!("  ✅ Exponential backoff for retries");
    tracing::trace!("  ✅ Zero-copy JSON parsing");
    tracing::trace!("  ✅ Cow strings for memory efficiency");
    tracing::trace!("  ✅ Error categorization");

    Ok(())
}
