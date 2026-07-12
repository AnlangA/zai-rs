#![no_main]

use libfuzzer_sys::fuzz_target;
use zai_rs::client::error::{ZaiError, contains_sensitive_info, mask_sensitive_info};

fuzz_target!(|input: &[u8]| {
    let message = String::from_utf8_lossy(input);
    let masked = mask_sensitive_info(&message);
    let _ = contains_sensitive_info(&masked);

    if let [status_hi, status_lo, code_hi, code_lo, ..] = input {
        let status = u16::from_be_bytes([*status_hi, *status_lo]);
        let code = u16::from_be_bytes([*code_hi, *code_lo]);
        let error = ZaiError::from_api_response(status, code, message.into_owned());
        let _ = (
            error.code(),
            error.message(),
            error.category(),
            error.compact(),
            error.is_retryable(),
        );
    }
});
