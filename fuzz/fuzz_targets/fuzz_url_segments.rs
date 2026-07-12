#![no_main]
use std::sync::OnceLock;

use libfuzzer_sys::fuzz_target;
use zai_rs::client::ApiFamily;
use zai_rs::client::endpoint::EndpointConfig;

static ENDPOINTS: OnceLock<EndpointConfig> = OnceLock::new();

fuzz_target!(|data: &[u8]| {
    let Ok(input) = std::str::from_utf8(data) else {
        return;
    };
    let endpoints = ENDPOINTS.get_or_init(|| {
        EndpointConfig::defaults().expect("built-in endpoint configuration must remain valid")
    });

    // NUL separates path components so one input can cover multi-segment URL
    // construction, including empty and traversal-like components.
    let segments = input.split('\0').take(8).collect::<Vec<_>>();
    let _ = endpoints.resolve(ApiFamily::PaasV4, &segments);
});
