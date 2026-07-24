//! Response-decoding helpers.
//!
//! [`extract_error_envelope`] is used by the buffered transport before returning
//! bytes or deserializing JSON. SSE handshakes additionally require the exact
//! event-stream media type.

use crate::{ZaiError, ZaiResult, client::error::codes};

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

/// Decide whether a body parses as a genuine business error envelope.
///
/// A flat `{code, message}` with a code other than `0` or `200` is an error; a
/// nested `{error}` is always an error. Both success codes are used by official
/// API families and must flow through to their typed response decoders.
#[derive(serde::Deserialize)]
#[serde(untagged)]
pub(crate) enum WireEnvelope {
    Nested {
        error: WireError,
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
pub(crate) struct WireError {
    code: serde_json::Value,
    #[serde(default)]
    message: Option<String>,
}

impl WireEnvelope {
    pub(crate) fn is_error(&self) -> bool {
        match self {
            WireEnvelope::Nested { .. } => true,
            WireEnvelope::Flat { code, .. } => !is_success_code(code),
        }
    }
}

fn is_success_code(code: &serde_json::Value) -> bool {
    match code {
        serde_json::Value::Number(n) => matches!(n.as_u64(), Some(0 | 200)),
        serde_json::Value::String(s) => matches!(s.as_str(), "0" | "200"),
        _ => false,
    }
}

/// Probe the body for a genuine business error envelope; return `true` if the
/// body parses as `{code, message}` with a code other than `0` or `200`, or as
/// `{error}`.
///
/// The parsed envelope type is crate-private; the Transport uses
/// [`extract_error_envelope`] to pull the code/message for error construction.
#[cfg(test)]
fn probe_error_envelope(body: &str) -> bool {
    serde_json::from_str::<WireEnvelope>(body)
        .ok()
        .is_some_and(|env| env.is_error())
}

/// Error code and message extracted from a recognized business envelope.
#[derive(Debug, Clone)]
pub struct BusinessError {
    /// Numeric, textual, or otherwise open-format business code.
    pub code: Option<serde_json::Value>,
    /// Human-readable service error message.
    pub message: String,
    /// Provider request identifier, when returned as a JSON string.
    pub request_id: Option<String>,
}

/// Parse and extract a genuine business error envelope's code/message, or
/// `None` if the body is a success / non-envelope.
pub fn extract_error_envelope(body: &str) -> Option<BusinessError> {
    let env = serde_json::from_str::<WireEnvelope>(body).ok()?;
    if !env.is_error() {
        return None;
    }
    match env {
        WireEnvelope::Nested { error, request_id } => Some(BusinessError {
            code: Some(error.code),
            message: error
                .message
                .unwrap_or_else(|| "API request failed".to_string()),
            request_id: request_id.and_then(|value| value.as_str().map(str::to_owned)),
        }),
        WireEnvelope::Flat {
            code,
            message,
            request_id,
        } => Some(BusinessError {
            code: Some(code),
            message: message.unwrap_or_else(|| "API request failed".to_string()),
            request_id: request_id.and_then(|value| value.as_str().map(str::to_owned)),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert!(probe_error_envelope(r#"{"code":500,"message":"x"}"#));
        assert!(probe_error_envelope(r#"{"code":500}"#));
        assert!(probe_error_envelope(
            r#"{"error":{"code":1302,"message":"x"}}"#
        ));
        assert!(probe_error_envelope(r#"{"error":{"code":1302}}"#));
        // Both official success-code conventions flow through.
        assert!(!probe_error_envelope(r#"{"code":0,"message":"ok"}"#));
        assert!(!probe_error_envelope(r#"{"code":"0","message":"ok"}"#));
        assert!(!probe_error_envelope(r#"{"code":200,"message":"ok"}"#));
        assert!(!probe_error_envelope(r#"{"code":"200","message":"ok"}"#));
        // Non-envelope body (chat success) does not match.
        assert!(!probe_error_envelope(r#"{"id":"x","choices":[]}"#));

        // Secret-shaped success data is ordinary payload, never an error
        // envelope merely because it resembles an API key.
        assert!(!probe_error_envelope(
            r#"{"model":"glm-5.2","key":"1234567890.abcdefghijklmnop"}"#
        ));

        for code in 300_u16..600 {
            let body = format!(r#"{{"code":{code},"message":"x"}}"#);
            assert!(probe_error_envelope(&body), "business code {code}");
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
}
