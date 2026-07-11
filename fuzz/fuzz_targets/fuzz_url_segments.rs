#![no_main]
use libfuzzer_sys::fuzz_target;
use zai_rs::client::v2::endpoint::EndpointConfig;
use zai_rs::client::v2::ApiFamily;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        if let Ok(ec) = EndpointConfig::defaults() {
            let _ = ec.resolve(ApiFamily::PaasV4, &[s]);
        }
    }
});
