//! Response decode pipeline (plan P03.4/P03.9).
//!
//! Order: probe the official error envelope → decode the private wire success
//! → validate the success invariant → convert to the public response. Content-
//! type is validated against the operation manifest (JSON/+json, SSE, or a
//! declared binary MIME).

use crate::{ZaiError, ZaiResult, client::error::codes};

/// Accepted JSON content-type prefixes for a typed-decode response.
pub const JSON_CONTENT_TYPES: &[&str] = &["application/json", "application/"];
/// The SSE content type.
pub const SSE_CONTENT_TYPE: &str = "text/event-stream";

/// Validate a response `Content-Type` against the expected kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpectedKind {
    Json,
    Sse,
    /// A specific binary MIME from the operation manifest.
    Binary(&'static str),
}

/// Validate the content-type. Mismatch → Protocol::UnexpectedContentType.
pub fn validate_content_type(raw: &str, expected: ExpectedKind) -> ZaiResult<()> {
    let lower = raw.to_ascii_lowercase();
    let ok = match expected {
        ExpectedKind::Json => JSON_CONTENT_TYPES
            .iter()
            .any(|p| lower == *p || lower.starts_with(p) || lower.ends_with("+json")),
        ExpectedKind::Sse => lower.starts_with(SSE_CONTENT_TYPE),
        ExpectedKind::Binary(mime) => {
            let m = mime.to_ascii_lowercase();
            lower == m || lower.starts_with(&m)
        },
    };
    if ok {
        Ok(())
    } else {
        Err(ZaiError::ApiError {
            code: codes::SDK_VALIDATION,
            message: format!("unexpected content-type: {raw:?} (expected {expected:?})"),
        })
    }
}

/// Decide whether a body parses as a genuine business error envelope.
///
/// Mirrors the P01.7 envelope-probe logic but lives here for the new Transport.
/// A flat `{code, message}` with `code != 200` is an error; a nested `{error}`
/// is always an error; `code == 200` (knowledge success) flows through.
#[derive(serde::Deserialize)]
#[serde(untagged)]
pub(crate) enum WireEnvelope {
    Nested {
        error: WireError,
    },
    Flat {
        code: serde_json::Value,
        message: String,
    },
}

#[derive(serde::Deserialize)]
pub(crate) struct WireError {
    code: serde_json::Value,
    #[allow(dead_code)]
    message: String,
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
        serde_json::Value::Number(n) => n.as_u64() == Some(200),
        serde_json::Value::String(s) => s == "200",
        _ => false,
    }
}

/// Probe the body for a genuine business error envelope; return `true` if the
/// body parses as `{code, message}` with `code != 200` or as `{error}`.
///
/// The parsed envelope type is crate-private; the Transport uses
/// [`extract_error_envelope`] to pull the code/message for error construction.
pub fn probe_error_envelope(body: &str) -> bool {
    serde_json::from_str::<WireEnvelope>(body)
        .ok()
        .is_some_and(|env| env.is_error())
}

/// Extracted business-error parts (public for the Transport's error builder).
#[derive(Debug, Clone)]
pub struct BusinessError {
    pub code: Option<serde_json::Value>,
    pub message: String,
}

/// Parse and extract a genuine business error envelope's code/message, or
/// `None` if the body is a success / non-envelope.
pub fn extract_error_envelope(body: &str) -> Option<BusinessError> {
    let env = serde_json::from_str::<WireEnvelope>(body).ok()?;
    if !env.is_error() {
        return None;
    }
    match env {
        WireEnvelope::Nested { error } => Some(BusinessError {
            code: Some(error.code),
            message: error.message,
        }),
        WireEnvelope::Flat { code, message } => Some(BusinessError {
            code: Some(code),
            message,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_content_type_accepted() {
        assert!(validate_content_type("application/json", ExpectedKind::Json).is_ok());
        assert!(
            validate_content_type("application/json; charset=utf-8", ExpectedKind::Json).is_ok()
        );
        assert!(validate_content_type("application/vnd.api+json", ExpectedKind::Json).is_ok());
        assert!(validate_content_type("text/plain", ExpectedKind::Json).is_err());
    }

    #[test]
    fn sse_content_type_accepted() {
        assert!(validate_content_type("text/event-stream", ExpectedKind::Sse).is_ok());
        assert!(
            validate_content_type("text/event-stream; charset=utf-8", ExpectedKind::Sse).is_ok()
        );
        assert!(validate_content_type("application/json", ExpectedKind::Sse).is_err());
    }

    #[test]
    fn binary_content_type_matches_manifest_mime() {
        assert!(validate_content_type("audio/pcm", ExpectedKind::Binary("audio/pcm")).is_ok());
        assert!(
            validate_content_type(
                "application/octet-stream",
                ExpectedKind::Binary("application/octet-stream")
            )
            .is_ok()
        );
        assert!(validate_content_type("text/html", ExpectedKind::Binary("audio/pcm")).is_err());
    }

    #[test]
    fn probe_finds_business_error_envelopes() {
        assert!(probe_error_envelope(r#"{"code":500,"message":"x"}"#));
        assert!(probe_error_envelope(
            r#"{"error":{"code":1302,"message":"x"}}"#
        ));
        // code == 200 is success, not an error envelope.
        assert!(!probe_error_envelope(r#"{"code":200,"message":"ok"}"#));
        // Non-envelope body (chat success) does not match.
        assert!(!probe_error_envelope(r#"{"id":"x","choices":[]}"#));
    }
}
