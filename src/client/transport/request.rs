//! Prepared HTTP requests consumed by the buffered transport.

use crate::client::RequestOptions;
use crate::client::transport::retry::RetrySafety;

/// One validated operation-local header whose value is always sensitive.
///
/// Unlike [`crate::client::AdditionalHeader`], this type is crate-private and
/// travels with one prepared operation only. Keeping the parsed header value
/// here means request construction cannot fail later while attempting to
/// reinterpret untrusted header text.
#[derive(Clone)]
pub(crate) struct SensitiveHeader {
    name: reqwest::header::HeaderName,
    value: reqwest::header::HeaderValue,
}

impl SensitiveHeader {
    /// Validate and own an operation-local header without retaining its raw
    /// value in any error message.
    pub(crate) fn new(name: &'static str, value: &str) -> crate::ZaiResult<Self> {
        if value.is_empty() || value.len() > 1024 {
            return Err(invalid_sensitive_header());
        }
        if !value.bytes().all(|byte| (0x21..=0x7e).contains(&byte)) {
            return Err(invalid_sensitive_header());
        }

        let name = reqwest::header::HeaderName::from_bytes(name.as_bytes())
            .map_err(|_| invalid_sensitive_header())?;
        let mut value = reqwest::header::HeaderValue::from_str(value)
            .map_err(|_| invalid_sensitive_header())?;
        value.set_sensitive(true);
        Ok(Self { name, value })
    }

    pub(crate) fn name(&self) -> &reqwest::header::HeaderName {
        &self.name
    }

    pub(crate) fn value(&self) -> &reqwest::header::HeaderValue {
        &self.value
    }
}

impl std::fmt::Debug for SensitiveHeader {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SensitiveHeader")
            .field("name", &self.name)
            .field("value", &"[REDACTED]")
            .finish()
    }
}

fn invalid_sensitive_header() -> crate::ZaiError {
    crate::ZaiError::ApiError {
        code: crate::client::error::codes::SDK_VALIDATION,
        message: "operation header value must be 1..=1024 printable ASCII bytes without whitespace"
            .to_string(),
    }
}

/// A request body kind the Transport knows how to encode and size-limit.
#[derive(Debug)]
pub enum BodyKind<'a> {
    /// No body (GET/DELETE).
    None,
    /// Raw bytes, typically containing JSON serialized before dispatch.
    Bytes(&'a bytes::Bytes),
    /// Multipart — built per attempt by a factory (files re-opened each try).
    Multipart(&'a super::multipart::MultipartBodyFactory),
}

/// How the final response body is consumed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResponseMode {
    /// Buffer JSON and require an `application/json` or `+json` success MIME.
    Json,
    /// Buffer a file download and require `application/octet-stream`.
    File,
    /// Buffer synthesized audio and accept only the documented audio MIME set.
    Audio,
}

impl ResponseMode {
    /// Value sent in the HTTP `Accept` header.
    pub const fn accept(self) -> &'static str {
        match self {
            Self::Json => "application/json",
            Self::File => "application/octet-stream",
            Self::Audio => "audio/wav, audio/x-wav, audio/pcm, application/octet-stream",
        }
    }

    /// Validate the media type of a successful buffered response.
    pub fn validate_content_type(self, raw: &str) -> crate::ZaiResult<()> {
        match self {
            Self::Json => super::decode::validate_json_content_type(raw),
            Self::File => {
                super::decode::validate_binary_content_type(raw, &["application/octet-stream"])
            },
            Self::Audio => super::decode::validate_binary_content_type(
                raw,
                &[
                    "audio/wav",
                    "audio/x-wav",
                    "audio/pcm",
                    "application/octet-stream",
                ],
            ),
        }
    }
}

/// A fully-prepared, validated request ready to be sent by the Transport.
pub struct PreparedRequest<'a> {
    /// Stable operation identifier from the canonical route registry.
    pub operation_id: &'static str,
    /// Uppercase HTTP method string.
    pub method: &'static str,
    /// Fully resolved request URL.
    pub url: String,
    /// Request body representation.
    pub body: BodyKind<'a>,
    /// Retry classification derived from the request method.
    pub retry_safety: RetrySafety,
    /// Transport-only overrides carried by the dispatching client handle.
    pub request_options: RequestOptions,
    /// HTTP statuses declared successful by the canonical operation contract.
    pub success_statuses: &'static [u16],
    /// Response buffering mode and associated size limit.
    pub response_mode: ResponseMode,
    /// Route template for tracing (never the materialized URL).
    pub route_template: String,
    /// Validated sensitive headers scoped to this operation only.
    pub(crate) sensitive_headers: Vec<SensitiveHeader>,
}

impl<'a> std::fmt::Debug for PreparedRequest<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Never print the body or full URL.
        f.debug_struct("PreparedRequest")
            .field("operation_id", &self.operation_id)
            .field("method", &self.method)
            .field("route", &self.route_template)
            .field("retry_safety", &self.retry_safety)
            .field("success_statuses", &self.success_statuses)
            .field("request_options", &self.request_options)
            .field("sensitive_header_count", &self.sensitive_headers.len())
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::SensitiveHeader;

    #[test]
    fn sensitive_header_validation_and_debug_never_expose_the_value() {
        let secret = "private-session-123";
        let header = SensitiveHeader::new("X-Session-Id", secret).unwrap();
        assert!(header.value().is_sensitive());
        let debug = format!("{header:?}");
        assert!(debug.contains("x-session-id"));
        assert!(!debug.contains(secret));

        for value in ["", "contains space", "contains\nnewline", "非 ASCII"] {
            let error = SensitiveHeader::new("X-Session-Id", value).unwrap_err();
            if !value.is_empty() {
                assert!(!error.to_string().contains(value));
            }
        }
        assert!(SensitiveHeader::new("X-Session-Id", &"x".repeat(1025)).is_err());
    }
}
