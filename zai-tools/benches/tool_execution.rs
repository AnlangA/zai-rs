//! Benchmarks for tool execution performance

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use serde::{Deserialize, Serialize};
use tokio::runtime::Runtime;
use zai_tools::prelude::*;

// Simple benchmark tool
#[derive(Debug, Clone, Serialize, Deserialize)]
struct BenchInput {
    value: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BenchOutput {
    result: i32,
}

impl ToolInput for BenchInput {}
impl ToolOutput for BenchOutput {}

#[derive(Clone)]
struct BenchTool {
    metadata: ToolMetadata,
}

impl BenchTool {
    fn new() -> Self {
        Self {
            metadata: ToolMetadata::new::<BenchInput, BenchOutput>(
                "bench_tool",
                "A tool for benchmarking"
            ),
        }
    }
}

#[async_trait]
impl Tool<BenchInput, BenchOutput> for BenchTool {
    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }
    
    async fn execute(&self, input: BenchInput) -> ToolResult<BenchOutput> {
        // Simple computation
        Ok(BenchOutput {
            result: input.value * 2,
        })
    }
}

// CPU-intensive tool for comparison
#[derive(Clone)]
struct CpuIntensiveTool {
    metadata: ToolMetadata,
}

impl CpuIntensiveTool {
    fn new() -> Self {
        Self {
            metadata: ToolMetadata::new::<BenchInput, BenchOutput>(
                "cpu_tool",
                "A CPU-intensive tool"
            ),
        }
    }
}

#[async_trait]
impl Tool<BenchInput, BenchOutput> for CpuIntensiveTool {
    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }
    
    async fn execute(&self, input: BenchInput) -> ToolResult<BenchOutput> {
        // Simulate CPU-intensive work
        let mut result = input.value;
        for _ in 0..1000 {
            result = (result * 17 + 31) % 1000000;
        }
        Ok(BenchOutput { result })
    }
}

fn bench_tool_execution(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let registry = ToolRegistry::builder()
        .with_tool(BenchTool::new())
        .unwrap()
        .build();
    
    let executor = ToolExecutor::builder(registry).build();
    
    c.bench_function("simple_tool_execution", |b| {
        b.iter(|| {
            rt.block_on(async {
                let input = serde_json::json!({"value": black_box(42)});
                let result = executor.execute("bench_tool", input).await.unwrap();
                black_box(result);
            })
        });
    });
}

fn bench_parallel_execution(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let registry = ToolRegistry::builder()
        .with_tool(BenchTool::new())
        .unwrap()
        .build();
    
    let executor = ToolExecutor::builder(registry).build();
    
    let mut group = c.benchmark_group("parallel_execution");
    
    for &count in &[1, 5, 10, 20, 50] {
        group.bench_with_input(BenchmarkId::new("tools", count), &count, |b, &count| {
            b.iter(|| {
                rt.block_on(async {
                    let requests: Vec<_> = (0..count)
                        .map(|i| {
                            (
                                "bench_tool".to_string(),
                                serde_json::json!({"value": black_box(i)}),
                            )
                        })
                        .collect();

                    let results = executor.execute_parallel(requests).await;
                    black_box(results);
                })
            });
        });
    }
    
    group.finish();
}

fn bench_registry_operations(c: &mut Criterion) {
    let registry = ToolRegistry::builder()
        .with_tool(BenchTool::new())
        .unwrap()
        .build();
    
    c.bench_function("registry_lookup", |b| {
        b.iter(|| {
            let metadata = registry.metadata(black_box("bench_tool"));
            black_box(metadata);
        });
    });
    
    c.bench_function("registry_has_tool", |b| {
        b.iter(|| {
            let has_tool = registry.has_tool(black_box("bench_tool"));
            black_box(has_tool);
        });
    });
    
    c.bench_function("registry_input_schema", |b| {
        b.iter(|| {
            let schema = registry.input_schema(black_box("bench_tool"));
            black_box(schema);
        });
    });
}

fn bench_cpu_intensive_vs_simple(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let simple_registry = ToolRegistry::builder()
        .with_tool(BenchTool::new())
        .unwrap()
        .build();
    
    let cpu_registry = ToolRegistry::builder()
        .with_tool(CpuIntensiveTool::new())
        .unwrap()
        .build();
    
    let simple_executor = ToolExecutor::builder(simple_registry).build();
    let cpu_executor = ToolExecutor::builder(cpu_registry).build();
    
    let mut group = c.benchmark_group("tool_comparison");
    
    group.bench_function("simple_tool", |b| {
        b.iter(|| {
            rt.block_on(async {
                let input = serde_json::json!({"value": black_box(42)});
                let result = simple_executor.execute("bench_tool", input).await.unwrap();
                black_box(result);
            })
        });
    });

    group.bench_function("cpu_intensive_tool", |b| {
        b.iter(|| {
            rt.block_on(async {
                let input = serde_json::json!({"value": black_box(42)});
                let result = cpu_executor.execute("cpu_tool", input).await.unwrap();
                black_box(result);
            })
        });
    });
    
    group.finish();
}

fn bench_error_handling(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let registry = ToolRegistry::new(); // Empty registry
    let executor = ToolExecutor::builder(registry).build();
    
    c.bench_function("tool_not_found_error", |b| {
        b.iter(|| {
            rt.block_on(async {
                let input = serde_json::json!({"value": black_box(42)});
                let result = executor.execute("nonexistent_tool", input).await.unwrap();
                black_box(result);
            })
        });
    });
}

criterion_group!(
    benches,
    bench_tool_execution,
    bench_parallel_execution,
    bench_registry_operations,
    bench_cpu_intensive_vs_simple,
    bench_error_handling
);
criterion_main!(benches);
