use std::{
    hint::black_box,
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use serde_json::json;
use zai_rs::{
    client::{ApiFamily, EndpointConfig, error::mask_sensitive_info},
    model::sse_parser::SseEventParser,
    realtime::audio::encode_wav_pcm_base64,
    toolkits::{CachePolicy, FunctionTool, RetryPolicy, ToolExecutionPolicy, ToolExecutor},
};

fn sse_parser(c: &mut Criterion) {
    let data = "x".repeat(64 * 1024);
    let payload = format!("data: {data}\n\n");
    let mut group = c.benchmark_group("sse_parser");
    group.throughput(Throughput::Bytes(payload.len() as u64));

    for chunk_size in [64, 1024, payload.len()] {
        group.bench_with_input(
            BenchmarkId::new("single_event", chunk_size),
            &chunk_size,
            |b, &chunk_size| {
                b.iter(|| {
                    let mut parser = SseEventParser::new();
                    let mut emitted = 0usize;
                    for chunk in payload.as_bytes().chunks(chunk_size) {
                        emitted += parser
                            .push(black_box(chunk))
                            .into_iter()
                            .map(|event| event.len())
                            .sum::<usize>();
                    }
                    emitted += parser
                        .finish()
                        .into_iter()
                        .map(|event| event.len())
                        .sum::<usize>();
                    black_box(emitted)
                });
            },
        );
    }
    group.finish();
}

fn make_redaction_input(target_len: usize) -> String {
    const LINE: &str =
        "request completed token=alpha.secretvalue123456 Authorization: Bearer secret-token ";
    let mut input = String::with_capacity(target_len);
    while input.len() < target_len {
        input.push_str(LINE);
    }
    input.truncate(target_len);
    input
}

fn make_clean_redaction_input(target_len: usize) -> String {
    const LINE: &str =
        "request completed successfully after validation and response decoding; diagnostics clean ";
    let mut input = String::with_capacity(target_len);
    while input.len() < target_len {
        input.push_str(LINE);
    }
    input.truncate(target_len);
    input
}

fn redaction(c: &mut Criterion) {
    let mut group = c.benchmark_group("sensitive_redaction");
    for size in [1024usize, 64 * 1024] {
        let input = make_redaction_input(size);
        group.throughput(Throughput::Bytes(input.len() as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &input, |b, input| {
            b.iter(|| mask_sensitive_info(black_box(input.as_str())));
        });
    }
    group.finish();

    let mut clean_group = c.benchmark_group("clean_redaction");
    for size in [1024usize, 64 * 1024] {
        let input = make_clean_redaction_input(size);
        clean_group.throughput(Throughput::Bytes(input.len() as u64));
        clean_group.bench_with_input(BenchmarkId::from_parameter(size), &input, |b, input| {
            b.iter(|| mask_sensitive_info(black_box(input.as_str())));
        });
    }
    clean_group.finish();
}

fn endpoint_resolution(c: &mut Criterion) {
    let endpoints = EndpointConfig::defaults().expect("official endpoint defaults must be valid");
    let mut group = c.benchmark_group("endpoint_resolution");

    group.bench_function("static_segments", |b| {
        b.iter(|| {
            endpoints.resolve(
                black_box(ApiFamily::PaasV4),
                black_box(&["chat", "completions"]),
            )
        });
    });
    group.bench_function("encoded_dynamic_segments", |b| {
        b.iter(|| {
            endpoints.resolve_with_query(
                black_box(ApiFamily::AgentV1),
                black_box(&["agents", "测试 / resource", "invoke"]),
                black_box(&[("conversation_id", "thread / 42")]),
            )
        });
    });
    group.finish();
}

fn cache_executor() -> ToolExecutor {
    let tool = FunctionTool::builder("echo", "Return the JSON input")
        .schema(json!({
            "type": "object",
            "properties": {"n": {"type": "integer"}},
            "required": ["n"]
        }))
        .execution_policy(ToolExecutionPolicy::new(
            CachePolicy::Pure,
            RetryPolicy::Never,
        ))
        .handler(|input| async move { Ok(input) })
        .build()
        .expect("benchmark tool definition must be valid");
    let executor = ToolExecutor::builder()
        .enable_cache()
        .cache_max_size(1)
        .build();
    executor
        .add_dyn_tool(Box::new(tool))
        .expect("benchmark tool registration must be unique");
    executor
}

fn tool_cache(c: &mut Criterion) {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Tokio benchmark runtime must build");
    let hit_executor = cache_executor();
    let hit_input = json!({"n": 7});
    runtime
        .block_on(hit_executor.execute("echo", hit_input.clone()))
        .expect("cache priming call must execute");

    let miss_executor = cache_executor();
    let next_input = AtomicU64::new(0);
    let mut group = c.benchmark_group("tool_cache");
    group.bench_function("hit", |b| {
        b.to_async(&runtime).iter(|| async {
            hit_executor
                .execute("echo", black_box(hit_input.clone()))
                .await
                .expect("cached tool call must execute")
        });
    });
    group.bench_function("miss_with_fifo_eviction", |b| {
        b.to_async(&runtime).iter(|| async {
            let n = next_input.fetch_add(1, Ordering::Relaxed);
            miss_executor
                .execute("echo", black_box(json!({"n": n})))
                .await
                .expect("uncached tool call must execute")
        });
    });
    group.finish();
}

fn realtime_wav_base64(c: &mut Criterion) {
    let mut group = c.benchmark_group("realtime_wav_base64");
    for size in [640usize, 64 * 1024] {
        let pcm = vec![0u8; size];
        group.throughput(Throughput::Bytes(pcm.len() as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &pcm, |b, pcm| {
            b.iter(|| {
                encode_wav_pcm_base64(black_box(pcm.as_slice()), black_box(16_000))
                    .expect("even-sized PCM benchmark input must encode")
            });
        });
    }
    group.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .warm_up_time(Duration::from_secs(1))
        .measurement_time(Duration::from_secs(3))
        .sample_size(30);
    targets = sse_parser, redaction, endpoint_resolution, tool_cache, realtime_wav_base64
}
criterion_main!(benches);
