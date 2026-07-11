#![no_main]
use libfuzzer_sys::fuzz_target;
use zai_rs::client::transport::decode::probe_error_envelope;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = probe_error_envelope(s);
    }
});
