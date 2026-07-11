#![no_main]
use libfuzzer_sys::fuzz_target;
use zai_rs::model::sse_parser::SseEventParser;

fuzz_target!(|data: &[u8]| {
    let mut p = SseEventParser::new();
    let _ = p.push(data);
});
