//! Sealed `RequestSpec` and `PreparedRequest` (plan P03.2).
//!
//! Every endpoint implements the crate-private sealed `RequestSpec` trait,
//! declaring the fixed method/path/content-type/retry-safety/limits/validator
//! and decoder. The Transport turns a `RequestSpec` into a `PreparedRequest`
//! (validated URL + method + body) and dispatches it.
//!
//! The trait is sealed so only crate-internal endpoints can implement it; the
//! public surface is the service facades, not raw specs.

use std::any::type_name;

use crate::client::RetryOverride;
use crate::client::transport::retry::RetrySafety;

/// A request body kind the Transport knows how to encode and size-limit.
#[derive(Debug)]
pub enum BodyKind<'a> {
    /// No body (GET/DELETE).
    None,
    /// A JSON-serializable body, serialized once and reused across attempts.
    Json(&'a serde_json::Value),
    /// Raw bytes (already serialized); reused via `Arc<Bytes>`.
    Bytes(&'a bytes::Bytes),
    /// Multipart — built per attempt by a factory (files re-opened each try).
    Multipart(&'a super::multipart::MultipartBodyFactory),
}

/// How the final response body is consumed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResponseMode {
    Json,
    Binary,
}

/// A fully-prepared, validated request ready to be sent by the Transport.
pub struct PreparedRequest<'a> {
    pub method: &'static str,
    pub url: String,
    pub body: BodyKind<'a>,
    pub retry_safety: RetrySafety,
    pub retry_override: Option<RetryOverride>,
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

/// The fixed specification of an endpoint (plan P03.2).
///
/// Associated items mirror plan §4: METHOD, API_FAMILY, PATH_TEMPLATE, content
/// types, retry safety, response mode, done marker and success invariant.
#[allow(unused)] // associated items are consumed by the Transport once endpoints migrate (P05).
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
