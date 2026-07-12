#![no_main]
use libfuzzer_sys::fuzz_target;
use zai_rs::model::sse_parser::SseEventParser;

fuzz_target!(|data: &[u8]| {
    let Some((&chunk_seed, payload)) = data.split_first() else {
        return;
    };

    // Exercise incremental parsing across deterministic transport boundaries;
    // the first byte controls chunking and the remainder remains arbitrary SSE.
    let chunk_size = usize::from(chunk_seed % 64) + 1;
    let mut parser = SseEventParser::new();
    for chunk in payload.chunks(chunk_size) {
        let _ = parser.push(chunk);
    }
    let _ = parser.finish();
});
