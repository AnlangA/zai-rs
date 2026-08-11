//! Deterministic allocation census for buffered-response business-error probes.
//!
//! Reserved `code` values are provider-controlled and open-format. The probe
//! must stream past composite values and bound retained strings rather than
//! materializing attacker-sized diagnostics before the typed response decode.

use std::{alloc::System, hint::black_box};

use stats_alloc::{INSTRUMENTED_SYSTEM, Region, StatsAlloc};

#[path = "../src/client/transport/decode/business_error.rs"]
mod business_error;

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

const NUMERIC_VALUES: usize = 1024 * 1024;
const LONG_CODE_BYTES: usize = 32 * 1024 * 1024;
const MAX_ALLOCATIONS: usize = 4;
const MAX_REALLOCATIONS: usize = 4;
const MAX_ALLOCATED_BYTES: usize = 1024;
const MAX_REALLOCATED_BYTES: isize = 1024;

#[derive(Clone, Copy)]
enum ExpectedCode {
    Array,
    ElidedString,
    Malformed,
}

fn assert_error_probe_contract() {
    assert!(business_error::is_success_code(&serde_json::json!(200)));
    let business_error::ProbeOutcome::Error(error) = business_error::probe_error_envelope(
        r#"{"code":1302,"message":"limited","request_id":"bench-request"}"#,
    ) else {
        panic!("known business-error envelope did not produce an error probe");
    };
    assert_eq!(error.code, Some(serde_json::json!(1302)));
    assert_eq!(error.message, "limited");
    assert_eq!(error.request_id.as_deref(), Some("bench-request"));

    // Cargo compiles a harness-free bench with `cfg(test)`, which also makes
    // the decode contract's legacy projection available in this included
    // module. Exercise it outside the measured region so all-target linting
    // remains clean while the census below still covers only production code.
    #[cfg(test)]
    assert!(
        business_error::extract_error_envelope(r#"{"code":1302}"#).is_some(),
        "legacy test projection stopped recognizing a business error"
    );
}

fn numeric_array_envelope(prefix: &str, suffix: &str) -> String {
    let mut body = String::with_capacity(prefix.len() + NUMERIC_VALUES * 2 + suffix.len());
    body.push_str(prefix);
    for index in 0..NUMERIC_VALUES {
        if index != 0 {
            body.push(',');
        }
        body.push('0');
    }
    body.push_str(suffix);
    assert!(body.len() >= 2 * 1024 * 1024);
    body
}

fn long_string_envelope() -> String {
    let mut body = String::with_capacity(LONG_CODE_BYTES + 16);
    body.push_str("{\"code\":\"");
    body.extend(std::iter::repeat_n('7', LONG_CODE_BYTES));
    body.push_str("\"}");
    assert!(body.len() >= LONG_CODE_BYTES);
    body
}

fn long_escaped_string_envelope() -> String {
    const ESCAPE: &str = "\\u0037";
    let repetitions = LONG_CODE_BYTES / ESCAPE.len();
    let mut body = String::with_capacity(repetitions * ESCAPE.len() + 16);
    body.push_str("{\"code\":\"");
    for _ in 0..repetitions {
        body.push_str(ESCAPE);
    }
    body.push_str("\"}");
    assert!(body.len() >= LONG_CODE_BYTES);
    body
}

fn long_numeric_envelope() -> String {
    let mut body = String::with_capacity(LONG_CODE_BYTES + 16);
    body.push_str("{\"code\":");
    body.extend(std::iter::repeat_n('7', LONG_CODE_BYTES));
    body.push('}');
    assert!(body.len() >= LONG_CODE_BYTES);
    body
}

fn deeply_nested_envelope() -> String {
    let depth = LONG_CODE_BYTES / 2;
    let mut body = String::with_capacity(depth * 2 + 16);
    body.push_str("{\"code\":");
    body.extend(std::iter::repeat_n('[', depth));
    body.push('0');
    body.extend(std::iter::repeat_n(']', depth));
    body.push('}');
    assert!(body.len() >= LONG_CODE_BYTES);
    body
}

fn census(
    scenario: &str,
    body: &str,
    numeric_values: usize,
    code_string_bytes: usize,
    expected_code: ExpectedCode,
) {
    let region = Region::new(GLOBAL);
    let probe = business_error::probe_error_envelope(black_box(body));
    black_box(&probe);
    let stats = region.change();

    match expected_code {
        ExpectedCode::Malformed => assert!(
            matches!(probe, business_error::ProbeOutcome::Malformed),
            "over-nested reserved code was not rejected before deserialization"
        ),
        ExpectedCode::Array | ExpectedCode::ElidedString => {
            let business_error::ProbeOutcome::Error(error) = &probe else {
                panic!("reserved code did not produce a business-error probe");
            };
            let Some(code) = error.code.as_ref() else {
                panic!("business-error probe dropped the reserved code");
            };
            assert!(
                match expected_code {
                    ExpectedCode::Array => code.as_array().is_some_and(Vec::is_empty),
                    ExpectedCode::ElidedString => code
                        .as_str()
                        .is_some_and(|value| value.len() <= 16 && value.parse::<u16>().is_err()),
                    ExpectedCode::Malformed => unreachable!(),
                },
                "reserved code did not retain only its bounded shape: {code}"
            );
        },
    }
    assert!(
        stats.allocations <= MAX_ALLOCATIONS,
        "business-error probe allocation count grew with reserved code: {stats:?}"
    );
    assert!(
        stats.bytes_allocated <= MAX_ALLOCATED_BYTES,
        "business-error probe materialized reserved code data: {stats:?}"
    );
    assert!(
        stats.reallocations <= MAX_REALLOCATIONS,
        "business-error probe repeatedly grew temporary storage: {stats:?}"
    );
    assert!(
        stats.bytes_reallocated <= MAX_REALLOCATED_BYTES,
        "business-error probe reallocated unknown response data: {stats:?}"
    );
    println!(
        "{{\"benchmark\":\"business_error_allocations\",\"scenario\":\"{scenario}\",\"payload_bytes\":{},\"numeric_values\":{numeric_values},\"code_string_bytes\":{code_string_bytes},\"allocations\":{},\"reallocations\":{},\"bytes_allocated\":{},\"bytes_reallocated\":{}}}",
        body.len(),
        stats.allocations,
        stats.reallocations,
        stats.bytes_allocated,
        stats.bytes_reallocated,
    );
}

fn main() {
    assert_error_probe_contract();

    let body = numeric_array_envelope("{\"code\":[", "]}");
    census(
        "top_level_numeric_code_array",
        &body,
        NUMERIC_VALUES,
        0,
        ExpectedCode::Array,
    );
    drop(body);

    let body = numeric_array_envelope("{\"error\":{\"code\":[", "]}}");
    census(
        "nested_numeric_code_array",
        &body,
        NUMERIC_VALUES,
        0,
        ExpectedCode::Array,
    );
    drop(body);

    let body = long_string_envelope();
    census(
        "top_level_long_code_string",
        &body,
        0,
        LONG_CODE_BYTES,
        ExpectedCode::ElidedString,
    );
    drop(body);

    let body = long_escaped_string_envelope();
    census(
        "top_level_long_escaped_code_string",
        &body,
        0,
        LONG_CODE_BYTES,
        ExpectedCode::ElidedString,
    );
    drop(body);

    let body = long_numeric_envelope();
    census(
        "top_level_long_numeric_code",
        &body,
        0,
        0,
        ExpectedCode::ElidedString,
    );
    drop(body);

    let body = deeply_nested_envelope();
    census(
        "top_level_deeply_nested_code",
        &body,
        0,
        0,
        ExpectedCode::Malformed,
    );
}
