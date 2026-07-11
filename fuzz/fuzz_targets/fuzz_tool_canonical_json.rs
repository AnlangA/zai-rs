#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Canonicalize arbitrary JSON without panicking.
    if let Ok(v) = serde_json::from_slice::<serde_json::Value>(data) {
        let _ = serde_json::to_string(&v);
    }
});
