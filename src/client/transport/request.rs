//! Prepared HTTP requests and a reserved sealed endpoint contract.
//!
//! Current endpoints construct [`PreparedRequest`] values directly. The sealed
//! [`RequestSpec`] trait describes a possible endpoint contract but currently
//! has no implementations and is not consumed by the transport.

use std::any::type_name;

use crate::client::RetryOverride;
use crate::client::transport::retry::RetrySafety;

/// A request body kind the Transport knows how to encode and size-limit.
#[derive(Debug)]
pub enum BodyKind<'a> {
    /// No body (GET/DELETE).
    None,
    /// A JSON value serialized whenever an attempt's request is built.
    Json(&'a serde_json::Value),
    /// Raw bytes, typically containing JSON serialized before dispatch.
    Bytes(&'a bytes::Bytes),
    /// Multipart — built per attempt by a factory (files re-opened each try).
    Multipart(&'a super::multipart::MultipartBodyFactory),
}

/// How the final response body is consumed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResponseMode {
    /// Buffer the response under the JSON response-size limit.
    Json,
    /// Buffer the response under the binary/file response-size limit.
    Binary,
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

/// Sealed module — only crate-internal types can implement `RequestSpec`.
mod private {
    pub trait Sealed {}
}

/// Reserved sealed specification for a fixed endpoint.
///
/// The current transport does not consume this trait; endpoints build
/// [`PreparedRequest`] directly. It remains sealed for potential crate-internal
/// adoption.
#[allow(unused)] // Reserved endpoint contract; current callers use PreparedRequest directly.
pub trait RequestSpec: private::Sealed {
    /// The Rust type name, for diagnostics.
    fn spec_name(&self) -> &'static str {
        type_name::<Self>()
    }
    /// HTTP method.
    const METHOD: &'static str;
    /// Route template (e.g. `"/paas/v4/files/{file_id}"`) for tracing.
    const PATH_TEMPLATE: &'static str;
    /// Retry safety (GET/HEAD/PUT/DELETE = Idempotent; POST/PATCH = NonIdempotent).
    const RETRY_SAFETY: RetrySafety;
    /// Request content type.
    const REQUEST_CONTENT_TYPE: &'static str;
    /// Expected success content type for typed decode validation.
    const ACCEPT: &'static str;
    /// Whether the success path requires `data: [DONE]` (SSE) or a binary stream.
    const REQUIRES_DONE: bool;
}
