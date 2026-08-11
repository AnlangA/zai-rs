//! Response-decoding helpers.
//!
//! The buffered and streaming transports use [`probe_error_envelope`] before
//! returning bytes or deserializing JSON. SSE handshakes additionally require
//! the exact event-stream media type.

use crate::{ZaiError, ZaiResult, client::error::codes};

mod business_error;

pub use business_error::BusinessError;
#[cfg(test)]
pub use business_error::extract_error_envelope;
#[cfg(test)]
pub(crate) use business_error::is_success_code;
pub(crate) use business_error::{ProbeOutcome, probe_error_envelope};

/// The SSE content type.
pub(crate) const SSE_CONTENT_TYPE: &str = "text/event-stream";

fn normalized_media_type(raw: &str) -> &str {
    raw.split(';').next().unwrap_or_default().trim()
}

fn unexpected_content_type(expected: &str) -> ZaiError {
    ZaiError::ApiError {
        code: codes::SDK_VALIDATION,
        message: format!("unexpected response content-type (expected {expected})"),
    }
}

/// Require the `text/event-stream` media type for an SSE handshake.
pub(crate) fn validate_sse_content_type(raw: &str) -> ZaiResult<()> {
    if normalized_media_type(raw).eq_ignore_ascii_case(SSE_CONTENT_TYPE) {
        Ok(())
    } else {
        Err(unexpected_content_type(SSE_CONTENT_TYPE))
    }
}

/// Require `application/json` or a registered structured `+json` media type.
pub(crate) fn validate_json_content_type(raw: &str) -> ZaiResult<()> {
    let media_type = normalized_media_type(raw);
    let valid = media_type.eq_ignore_ascii_case("application/json")
        || media_type
            .rsplit_once('/')
            .is_some_and(|(_, subtype)| subtype.to_ascii_lowercase().ends_with("+json"));
    if valid {
        Ok(())
    } else {
        Err(unexpected_content_type("application/json or +json"))
    }
}

/// Require one of the endpoint's documented binary response media types.
pub(crate) fn validate_binary_content_type(raw: &str, allowed: &[&str]) -> ZaiResult<()> {
    let media_type = normalized_media_type(raw);
    if allowed
        .iter()
        .any(|expected| media_type.eq_ignore_ascii_case(expected))
    {
        Ok(())
    } else {
        Err(unexpected_content_type("a documented binary media type"))
    }
}

/// Probe the body for a genuine business error envelope; return `true` if the
/// body parses as `{code, message}` with a code other than `0` or `200`, or as
/// `{error}`.
///
/// This compatibility predicate exists only for the legacy projection tests;
/// production transport paths use the tri-state probe directly.
#[cfg(test)]
fn legacy_probe_error_envelope(body: &str) -> bool {
    extract_error_envelope(body).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(serde::Deserialize)]
    #[serde(untagged)]
    enum LegacyWireEnvelope {
        Nested {
            error: LegacyWireError,
            #[serde(default)]
            request_id: Option<serde_json::Value>,
        },
        Flat {
            code: serde_json::Value,
            #[serde(default)]
            message: Option<String>,
            #[serde(default)]
            request_id: Option<serde_json::Value>,
        },
    }

    #[derive(serde::Deserialize)]
    struct LegacyWireError {
        code: serde_json::Value,
        #[serde(default)]
        message: Option<String>,
    }

    fn legacy_extract_error_envelope(
        body: &str,
    ) -> Option<(serde_json::Value, String, Option<String>)> {
        let envelope = serde_json::from_str::<LegacyWireEnvelope>(body).ok()?;
        match envelope {
            LegacyWireEnvelope::Nested { error, request_id } => Some((
                business_error::project_legacy_code_for_transport_observation(error.code),
                error
                    .message
                    .unwrap_or_else(|| "API request failed".to_owned()),
                request_id.and_then(|value| value.as_str().map(str::to_owned)),
            )),
            LegacyWireEnvelope::Flat {
                code,
                message,
                request_id,
            } if !is_success_code(&code) => Some((
                business_error::project_legacy_code_for_transport_observation(code),
                message.unwrap_or_else(|| "API request failed".to_owned()),
                request_id.and_then(|value| value.as_str().map(str::to_owned)),
            )),
            LegacyWireEnvelope::Flat { .. } => None,
        }
    }

    fn projected_error(
        error: Option<BusinessError>,
    ) -> Option<(serde_json::Value, String, Option<String>)> {
        error.map(|error| (error.code.unwrap(), error.message, error.request_id))
    }

    #[test]
    fn sse_content_type_accepted() {
        assert!(validate_sse_content_type("text/event-stream").is_ok());
        assert!(validate_sse_content_type("text/event-stream; charset=utf-8").is_ok());
        assert!(validate_sse_content_type("application/json").is_err());
    }

    #[test]
    fn json_content_type_accepts_json_and_structured_suffix_only() {
        assert!(validate_json_content_type("application/json").is_ok());
        assert!(validate_json_content_type("Application/Problem+JSON; charset=utf-8").is_ok());
        assert!(validate_json_content_type("text/plain").is_err());
        assert!(validate_json_content_type("").is_err());
    }

    #[test]
    fn binary_content_type_uses_the_endpoint_allowlist() {
        assert!(validate_binary_content_type("audio/wav; rate=24000", &["audio/wav"]).is_ok());
        assert!(validate_binary_content_type("audio/mpeg", &["audio/wav"]).is_err());
    }

    #[test]
    fn probe_finds_business_error_envelopes() {
        assert!(legacy_probe_error_envelope(r#"{"code":500,"message":"x"}"#));
        assert!(legacy_probe_error_envelope(r#"{"code":500}"#));
        assert!(legacy_probe_error_envelope(
            r#"{"error":{"code":1302,"message":"x"}}"#
        ));
        assert!(legacy_probe_error_envelope(r#"{"error":{"code":1302}}"#));
        // Both official success-code conventions flow through.
        assert!(!legacy_probe_error_envelope(r#"{"code":0,"message":"ok"}"#));
        assert!(!legacy_probe_error_envelope(
            r#"{"code":"0","message":"ok"}"#
        ));
        assert!(!legacy_probe_error_envelope(
            r#"{"code":200,"message":"ok"}"#
        ));
        assert!(!legacy_probe_error_envelope(
            r#"{"code":"200","message":"ok"}"#
        ));
        // Non-envelope body (chat success) does not match.
        assert!(!legacy_probe_error_envelope(r#"{"id":"x","choices":[]}"#));

        // Secret-shaped success data is ordinary payload, never an error
        // envelope merely because it resembles an API key.
        assert!(!legacy_probe_error_envelope(
            r#"{"model":"glm-5.2","key":"1234567890.abcdefghijklmnop"}"# // gitleaks:allow -- synthetic test credential
        ));

        for code in 300_u16..600 {
            let body = format!(r#"{{"code":{code},"message":"x"}}"#);
            assert!(legacy_probe_error_envelope(&body), "business code {code}");
        }
    }

    #[test]
    fn error_envelopes_retain_only_string_request_ids() {
        let flat =
            extract_error_envelope(r#"{"code":1302,"message":"slow","request_id":"request-42"}"#)
                .unwrap();
        assert_eq!(flat.request_id.as_deref(), Some("request-42"));

        let nested = extract_error_envelope(
            r#"{"error":{"code":1302,"message":"slow"},"request_id":"request-43"}"#,
        )
        .unwrap();
        assert_eq!(nested.request_id.as_deref(), Some("request-43"));

        let non_string =
            extract_error_envelope(r#"{"code":1302,"request_id":{"unexpected":true}}"#).unwrap();
        assert_eq!(non_string.request_id, None);
    }

    #[test]
    fn envelope_probe_preserves_nested_first_and_flat_fallback_semantics() {
        let nested = extract_error_envelope(
            r#"{"error":{"code":1302,"message":"nested"},"code":500,"message":{"ignored":true}}"#,
        )
        .unwrap();
        assert_eq!(nested.code, Some(serde_json::json!(1302)));
        assert_eq!(nested.message, "nested");

        for body in [
            r#"{"error":"invalid","code":1303,"message":"flat"}"#,
            r#"{"error":{"message":"missing code"},"code":1303,"message":"flat"}"#,
            r#"{"error":{"code":1302,"message":7},"code":1303,"message":"flat"}"#,
            r#"{"error":{"code":1,"code":2},"code":1303,"message":"flat"}"#,
            r#"{"error":{"code":1},"error":{"code":2},"code":1303,"message":"flat"}"#,
        ] {
            let flat = extract_error_envelope(body).unwrap();
            assert_eq!(flat.code, Some(serde_json::json!(1303)), "{body}");
            assert_eq!(flat.message, "flat", "{body}");
        }

        for body in [
            r#"{"error":"invalid"}"#,
            r#"{"error":"invalid","code":200}"#,
            r#"{"code":1303,"message":{"not":"a string"}}"#,
            r#"{"code":1303,"request_id":"one","request_id":"two"}"#,
            r#"["code",1303]"#,
            r#"{"code":1303"#,
        ] {
            assert!(extract_error_envelope(body).is_none(), "{body}");
        }
    }

    #[test]
    fn nested_envelopes_ignore_flat_only_field_conflicts() {
        for body in [
            r#"{"error":{"code":1302},"code":1,"code":2}"#,
            r#"{"error":{"code":1302},"message":1,"message":2}"#,
        ] {
            let error = extract_error_envelope(body).unwrap();
            assert_eq!(error.code, Some(serde_json::json!(1302)), "{body}");
            assert_eq!(error.message, "API request failed", "{body}");
        }
    }

    #[test]
    fn strict_transport_probe_rejects_reserved_duplicates_only() {
        for body in [
            r#"{"code":1302,"code":200}"#,
            r#"{"error":{"code":1302},"error":{"code":200}}"#,
            r#"{"code":1302,"message":"one","message":"two"}"#,
            r#"{"code":1302,"request_id":"one","request_id":"two"}"#,
            r#"{"error":{"code":1302,"code":200}}"#,
            r#"{"error":{"code":1302,"message":"one","message":"two"}}"#,
            r#"{"error":{"code":1302},"code":1,"code":2}"#,
        ] {
            assert!(
                matches!(probe_error_envelope(body), ProbeOutcome::Ambiguous),
                "reserved duplicate was not ambiguous: {body}"
            );
        }

        assert!(matches!(
            probe_error_envelope(r#"{"error":"invalid","code":1303,"message":"flat"}"#),
            ProbeOutcome::Error(BusinessError { code: Some(code), .. })
                if code == serde_json::json!(1303)
        ));
        assert!(matches!(
            probe_error_envelope(
                r#"{"id":"ordinary-success","metadata":{"role":"safe","role":"admin"}}"#
            ),
            ProbeOutcome::Clean
        ));
    }

    #[test]
    fn streaming_probe_matches_legacy_transport_observable_behavior() {
        for body in [
            r#"{"code":1302,"message":"flat","request_id":"flat-id"}"#,
            r#"{"error":{"code":1302,"message":"nested"},"request_id":"nested-id"}"#,
            r#"{"code":0,"message":"ok"}"#,
            r#"{"code":"200"}"#,
            r#"{"error":{"code":null}}"#,
            r#"{"code":{"future":"shape"}}"#,
            r#"{"error":{"code":[1,2]}}"#,
            &format!(r#"{{"code":"{}"}}"#, "7".repeat(129)),
            r#"{"error":"invalid","code":1303,"message":"flat fallback"}"#,
            r#"{"error":{"message":"missing code"},"code":1303}"#,
            r#"{"error":{"code":1302,"message":7},"code":1303}"#,
            r#"{"error":{"code":1302},"code":1,"code":2}"#,
            r#"{"error":{"code":1302},"message":1,"message":2}"#,
            r#"{"error":{"code":1},"error":{"code":2},"code":1303}"#,
            r#"{"error":{"code":1,"code":2},"code":1303}"#,
            r#"{"code":1303,"code":1304}"#,
            r#"{"code":1303,"message":{"not":"text"}}"#,
            r#"{"code":1303,"request_id":{"future":"shape"}}"#,
            r#"{"code":1303,"request_id":"one","request_id":"two"}"#,
            r#"{"unknown":[0,0,0],"id":"ordinary-success"}"#,
            r#"{ "c\u006fde": 1302, "mess\u0061ge": "escaped" }"#,
            r#"{}"#,
            r#"[]"#,
            r#"null"#,
            r#"{"code":1303"#,
            r#"{"code":1303} trailing"#,
        ] {
            assert_eq!(
                projected_error(extract_error_envelope(body)),
                legacy_extract_error_envelope(body),
                "{body}"
            );
        }
    }

    #[test]
    fn strict_probe_distinguishes_malformed_json_from_valid_non_envelopes() {
        for body in [r#"{"code":1303"#, r#"{"code":1303} trailing"#, "["] {
            assert!(
                matches!(probe_error_envelope(body), ProbeOutcome::Malformed),
                "{body}"
            );
        }

        for body in ["null", "[]", r#"[0,{"code":1303}]"#] {
            assert!(
                matches!(probe_error_envelope(body), ProbeOutcome::Clean),
                "{body}"
            );
        }
    }
}
