//! Deterministic allocation census for the public incremental SSE parser.
//!
//! This deliberately does not measure wall-clock latency. Shared CI runners
//! are suitable for recording allocation counts/bytes, while timing trends
//! remain in the Criterion benchmark where they are reviewed rather than used
//! as a noisy hard gate.

use std::{alloc::System, hint::black_box};

use stats_alloc::{INSTRUMENTED_SYSTEM, Region, StatsAlloc};
use zai_rs::model::sse_parser::SseEventParser;

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

const PAYLOAD_SIZES: [usize; 2] = [64 * 1024, 8 * 1024 * 1024];
const CHUNK_SIZES: [usize; 3] = [64, 1024, 64 * 1024];
// These deliberately loose, deterministic ceilings catch per-chunk return
// allocations and whole-payload copies without turning allocator growth
// details into a brittle exact snapshot. The current implementation uses four
// or five allocations; the headroom accommodates standard-library growth-
// policy changes across the supported toolchains.
const MAX_ALLOCATIONS: usize = 16;
const MAX_REALLOCATIONS: usize = 64;
const MAX_ALLOCATED_BYTES_PER_PAYLOAD: usize = 4;
const MAX_REALLOCATED_BYTES_PER_PAYLOAD: usize = 3;
const MANY_LINE_COUNT: usize = 4096;
const MAX_MANY_LINE_ALLOCATIONS: usize = 16;
const MAX_MANY_LINE_REALLOCATIONS: usize = 32;

fn measure(payload_size: usize, chunk_size: usize) {
    let data = "x".repeat(payload_size);
    let wire = format!("data: {data}\n\n");
    let region = Region::new(GLOBAL);
    let mut parser = SseEventParser::new();
    let mut events = Vec::new();

    for chunk in wire.as_bytes().chunks(chunk_size) {
        events.extend(
            parser
                .try_push(black_box(chunk))
                .expect("bounded benchmark payload must parse"),
        );
    }
    events.extend(
        parser
            .try_finish()
            .expect("complete benchmark payload must finish"),
    );

    assert_eq!(
        events.len(),
        1,
        "benchmark emitted an unexpected event count"
    );
    assert_eq!(
        events[0].len(),
        payload_size,
        "benchmark payload was changed"
    );
    black_box(&events);

    let stats = region.change();
    assert!(
        stats.allocations <= MAX_ALLOCATIONS,
        "SSE allocation count scales with transport chunks: {stats:?}"
    );
    assert!(
        stats.reallocations <= MAX_REALLOCATIONS,
        "SSE reallocation count exceeded the logarithmic-growth budget: {stats:?}"
    );
    assert!(
        stats.bytes_allocated <= payload_size * MAX_ALLOCATED_BYTES_PER_PAYLOAD,
        "SSE parser allocated too many bytes for one payload: {stats:?}"
    );
    assert!(
        stats.bytes_reallocated
            <= isize::try_from(payload_size * MAX_REALLOCATED_BYTES_PER_PAYLOAD)
                .expect("benchmark payload budget fits in isize"),
        "SSE parser reallocated too many bytes for one payload: {stats:?}"
    );
    println!(
        "{{\"benchmark\":\"sse_allocations\",\"scenario\":\"single_line\",\"payload_bytes\":{payload_size},\"chunk_bytes\":{chunk_size},\"data_lines\":1,\"allocations\":{},\"reallocations\":{},\"bytes_allocated\":{},\"bytes_reallocated\":{}}}",
        stats.allocations, stats.reallocations, stats.bytes_allocated, stats.bytes_reallocated,
    );
}

fn measure_many_line_json() {
    let mut wire = Vec::new();
    wire.extend_from_slice(b"data: [\r\n");
    for value in 0..MANY_LINE_COUNT - 2 {
        wire.extend_from_slice(b"data: ");
        wire.extend_from_slice(value.to_string().as_bytes());
        if value + 1 != MANY_LINE_COUNT - 2 {
            wire.push(b',');
        }
        wire.extend_from_slice(if value % 2 == 0 { b"\n" } else { b"\r\n" });
    }
    wire.extend_from_slice(b"data: ]\r\n\r\n");

    let region = Region::new(GLOBAL);
    let mut parser = SseEventParser::new();
    let events = parser
        .try_push(black_box(&wire))
        .expect("bounded many-line benchmark payload must parse");
    let stats = region.change();

    assert_eq!(events.len(), 1, "many-line event was not emitted once");
    assert_eq!(
        events[0].split(|byte| *byte == b'\n').count(),
        MANY_LINE_COUNT,
        "data lines were not joined with exactly one newline"
    );
    let values: Vec<usize> =
        serde_json::from_slice(&events[0]).expect("joined many-line event must remain valid JSON");
    assert_eq!(values.len(), MANY_LINE_COUNT - 2);
    black_box(&events);

    println!(
        "{{\"benchmark\":\"sse_allocations\",\"scenario\":\"many_line_json\",\"payload_bytes\":{},\"chunk_bytes\":{},\"data_lines\":{MANY_LINE_COUNT},\"allocations\":{},\"reallocations\":{},\"bytes_allocated\":{},\"bytes_reallocated\":{}}}",
        events[0].len(),
        wire.len(),
        stats.allocations,
        stats.reallocations,
        stats.bytes_allocated,
        stats.bytes_reallocated,
    );

    assert!(
        stats.allocations <= MAX_MANY_LINE_ALLOCATIONS,
        "SSE allocation count scales with data lines: {stats:?}"
    );
    assert!(
        stats.reallocations <= MAX_MANY_LINE_REALLOCATIONS,
        "SSE many-line reallocation count exceeded the logarithmic-growth budget: {stats:?}"
    );
    assert!(
        stats.bytes_allocated <= wire.len() + events[0].len() * 2,
        "SSE many-line parser allocated too many input/payload bytes: {stats:?}"
    );
    assert!(
        stats.bytes_reallocated
            <= isize::try_from(events[0].len() * MAX_REALLOCATED_BYTES_PER_PAYLOAD)
                .expect("many-line benchmark payload budget fits in isize"),
        "SSE many-line parser reallocated too many payload bytes: {stats:?}"
    );
}

fn main() {
    for payload_size in PAYLOAD_SIZES {
        for chunk_size in CHUNK_SIZES {
            measure(payload_size, chunk_size);
        }
    }
    measure_many_line_json();
}
