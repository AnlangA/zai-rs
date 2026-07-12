//! Prepared HTTP requests consumed by the buffered transport.

use crate::client::RetryOverride;
use crate::client::transport::retry::RetrySafety;

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
    /// Uppercase HTTP method string.
    pub method: &'static str,
    /// Fully resolved request URL.
    pub url: String,
    /// Request body representation.
    pub body: BodyKind<'a>,
    /// Retry classification derived from the request method.
    pub retry_safety: RetrySafety,
    /// Optional caller assertion that the operation is idempotent.
    pub retry_override: Option<RetryOverride>,
    /// Response buffering mode and associated size limit.
    pub response_mode: ResponseMode,
    /// Route template for tracing (never the materialized URL).
    pub route_template: &'static str,
}

impl<'a> std::fmt::Debug for PreparedRequest<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Never print the body or full URL.
        f.debug_struct("PreparedRequest")
            .field("method", &self.method)
            .field("route", &self.route_template)
            .field("retry_safety", &self.retry_safety)
            .field("retry_override", &self.retry_override.is_some())
            .finish_non_exhaustive()
    }
}
