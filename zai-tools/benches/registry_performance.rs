//! Benchmarks for registry performance with many tools

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use serde::{Deserialize, Serialize};
use zai_tools::prelude::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SimpleInput {
    id: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SimpleOutput {
    result: u32,
}

impl ToolInput for SimpleInput {}
impl ToolOutput for SimpleOutput {}

#[derive(Clone)]
struct NumberedTool {
    metadata: ToolMetadata,
    id: u32,
}

impl NumberedTool {
    fn new(id: u32) -> Self {
        Self {
            metadata: ToolMetadata::new::<SimpleInput, SimpleOutput>(
                format!("tool_{}", id),
                format!("Tool number {}", id)
            ),
            id,
        }
    }
}

#[async_trait]
impl Tool<SimpleInput, SimpleOutput> for NumberedTool {
    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }
    
    async fn execute(&self, input: SimpleInput) -> ToolResult<SimpleOutput> {
        Ok(SimpleOutput {
            result: input.id + self.id,
        })
    }
}

fn create_registry_with_tools(count: usize) -> ToolRegistry {
    let mut builder = ToolRegistry::builder();
    
    for i in 0..count {
        builder = builder.with_tool(NumberedTool::new(i as u32)).unwrap();
    }
    
    builder.build()
}

fn bench_registry_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("registry_creation");
    
    for &count in &[10, 50, 100, 500, 1000] {
        group.bench_with_input(BenchmarkId::new("tools", count), &count, |b, &count| {
            b.iter(|| {
                let registry = create_registry_with_tools(black_box(count));
                black_box(registry);
            });
        });
    }
    
    group.finish();
}

fn bench_registry_lookup_performance(c: &mut Criterion) {
    let registries: Vec<_> = [10, 50, 100, 500, 1000]
        .iter()
        .map(|&count| (count, create_registry_with_tools(count)))
        .collect();
    
    let mut group = c.benchmark_group("registry_lookup");
    
    for (count, registry) in &registries {
        group.bench_with_input(BenchmarkId::new("tools", count), count, |b, &count| {
            b.iter(|| {
                // Look up a tool in the middle
                let tool_name = format!("tool_{}", count / 2);
                let metadata = registry.metadata(black_box(&tool_name));
                black_box(metadata);
            });
        });
    }
    
    group.finish();
}

fn bench_registry_iteration(c: &mut Criterion) {
    let registries: Vec<_> = [10, 50, 100, 500, 1000]
        .iter()
        .map(|&count| (count, create_registry_with_tools(count)))
        .collect();
    
    let mut group = c.benchmark_group("registry_iteration");
    
    for (count, registry) in &registries {
        group.bench_with_input(BenchmarkId::new("tools", count), count, |b, _count| {
            b.iter(|| {
                let tool_names = registry.tool_names();
                black_box(tool_names);
            });
        });
    }
    
    group.finish();
}

fn bench_registry_filtering(c: &mut Criterion) {
    // Create registries with tools that have different tags
    let create_tagged_registry = |count: usize| {
        let mut builder = ToolRegistry::builder();
        
        for i in 0..count {
            let tool = NumberedTool::new(i as u32);
            builder = builder.with_tool(tool).unwrap();
        }
        
        builder.build()
    };
    
    let registries: Vec<_> = [10, 50, 100, 500, 1000]
        .iter()
        .map(|&count| (count, create_tagged_registry(count)))
        .collect();
    
    let mut group = c.benchmark_group("registry_filtering");
    
    for (count, registry) in &registries {
        group.bench_with_input(BenchmarkId::new("tools", count), count, |b, _count| {
            b.iter(|| {
                // Filter tools (this would need to be implemented in the registry)
                let all_tools = registry.tool_names();
                let filtered: Vec<_> = all_tools
                    .iter()
                    .filter(|name| name.contains("tool_1"))
                    .collect();
                black_box(filtered);
            });
        });
    }
    
    group.finish();
}

fn bench_concurrent_registry_access(c: &mut Criterion) {
    use std::sync::Arc;
    use tokio::runtime::Runtime;
    
    let rt = Runtime::new().unwrap();
    let registry = Arc::new(create_registry_with_tools(100));
    
    c.bench_function("concurrent_registry_access", |b| {
        b.iter(|| {
            rt.block_on(async {
                let registry = registry.clone();

                let tasks: Vec<_> = (0..10)
                    .map(|i| {
                        let registry = registry.clone();
                        tokio::spawn(async move {
                            let tool_name = format!("tool_{}", i * 10);
                            let metadata = registry.metadata(&tool_name);
                            black_box(metadata);
                        })
                    })
                    .collect();

                for task in tasks {
                    task.await.unwrap();
                }
            })
        });
    });
}

fn bench_registry_memory_usage(c: &mut Criterion) {
    let mut group = c.benchmark_group("registry_memory");
    
    for &count in &[100, 500, 1000, 5000] {
        group.bench_with_input(BenchmarkId::new("tools", count), &count, |b, &count| {
            b.iter(|| {
                let registry = create_registry_with_tools(black_box(count));
                
                // Perform some operations to ensure the registry is used
                let tool_names = registry.tool_names();
                let first_tool = tool_names.first().unwrap();
                let metadata = registry.metadata(first_tool);
                
                black_box((registry, metadata));
            });
        });
    }
    
    group.finish();
}

criterion_group!(
    benches,
    bench_registry_creation,
    bench_registry_lookup_performance,
    bench_registry_iteration,
    bench_registry_filtering,
    bench_concurrent_registry_access,
    bench_registry_memory_usage
);
criterion_main!(benches);
