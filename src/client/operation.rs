//! Internal operation execution boundary.
//!
//! Public request types describe API semantics. This module owns the shared
//! mechanics that connect those types to canonical routes and the transport,
//! keeping endpoint resolution and response-mode selection out of business
//! modules.

use crate::client::ZaiClient;
use crate::client::routes::Route;
use crate::client::transport::multipart::MultipartBodyFactory;
use crate::client::transport::request::{BodyKind, PreparedRequest, ResponseMode, SensitiveHeader};
use crate::client::transport::retry::RetrySafety;
use crate::{ZaiError, ZaiResult};
use serde::Serialize;

/// One canonical API operation plus its value-bearing path and query inputs.
///
/// Values are owned only until URL resolution. Diagnostic metadata is derived
/// from the route definition and therefore cannot expose those values.
pub(crate) struct Operation<'client> {
    client: &'client ZaiClient,
    route: Route,
    parameters: Vec<String>,
    query: Vec<(String, String)>,
    sensitive_headers: Vec<SensitiveHeader>,
}

impl ZaiClient {
    /// Start a typed dispatch through the canonical operation boundary.
    pub(crate) fn operation(&self, route: Route) -> Operation<'_> {
        Operation::new(self, route)
    }
}

impl<'client> Operation<'client> {
    fn new(client: &'client ZaiClient, route: Route) -> Self {
        Self {
            client,
            route,
            parameters: Vec::new(),
            query: Vec::new(),
            sensitive_headers: Vec::new(),
        }
    }

    /// Supply dynamic path segments in route-declaration order.
    pub(crate) fn with_parameters<I, S>(mut self, parameters: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.parameters
            .extend(parameters.into_iter().map(Into::into));
        self
    }

    /// Serialize a flat scalar query object without exposing values to trace
    /// metadata.
    pub(crate) fn with_query<T>(mut self, query: &T) -> ZaiResult<Self>
    where
        T: Serialize + ?Sized,
    {
        crate::client::query::extend(&mut self.query, query)?;
        Ok(self)
    }

    /// Attach one validated sensitive header to this operation only.
    ///
    /// The value is parsed and marked sensitive before request preparation;
    /// neither diagnostics nor validation errors retain the supplied text.
    pub(crate) fn with_sensitive_header(
        mut self,
        name: &'static str,
        value: &str,
    ) -> ZaiResult<Self> {
        let header = SensitiveHeader::new(name, value)?;
        if self
            .sensitive_headers
            .iter()
            .any(|existing| existing.name() == header.name())
        {
            return Err(ZaiError::ApiError {
                code: crate::client::error::codes::SDK_VALIDATION,
                message: "operation header must not be configured more than once".to_string(),
            });
        }
        self.sensitive_headers.push(header);
        Ok(self)
    }

    fn resolve(&self) -> ZaiResult<String> {
        let parameters = self
            .parameters
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>();
        let query = self
            .query
            .iter()
            .map(|(key, value)| (key.as_str(), value.as_str()))
            .collect::<Vec<_>>();
        self.client
            .endpoints()
            .resolve_route_with_query(self.route, &parameters, &query)
    }

    fn prepare<'body>(
        &self,
        url: String,
        body: BodyKind<'body>,
        response_mode: ResponseMode,
        retry_safety: RetrySafety,
    ) -> PreparedRequest<'body> {
        PreparedRequest {
            operation_id: self.route.operation_id(),
            method: self.route.method(),
            url,
            body,
            retry_safety,
            request_options: self.client.request_options,
            success_statuses: self.route.success_statuses(),
            response_mode,
            route_template: self.route.trace_template(),
            sensitive_headers: self.sensitive_headers.clone(),
        }
    }

    /// Serialize a JSON body, dispatch it, and decode a JSON response.
    pub(crate) async fn send_json<B, R>(self, body: &B) -> ZaiResult<R>
    where
        B: serde::Serialize + ?Sized,
        R: serde::de::DeserializeOwned,
    {
        let url = self.resolve()?;
        let bytes = bytes::Bytes::from(serde_json::to_vec(body).map_err(ZaiError::from)?);
        let request = self.prepare(
            url,
            BodyKind::Bytes(&bytes),
            ResponseMode::Json,
            RetrySafety::for_method(self.route.method()),
        );
        self.client.inner.sender.send(&request).await?.json()
    }

    /// Dispatch a body-less operation and decode a JSON response.
    pub(crate) async fn send_empty<R>(self) -> ZaiResult<R>
    where
        R: serde::de::DeserializeOwned,
    {
        let url = self.resolve()?;
        let request = self.prepare(
            url,
            BodyKind::None,
            ResponseMode::Json,
            RetrySafety::for_method(self.route.method()),
        );
        self.client.inner.sender.send(&request).await?.json()
    }

    /// Serialize a JSON body and return a validated audio payload.
    pub(crate) async fn send_json_bytes<B>(self, body: &B) -> ZaiResult<bytes::Bytes>
    where
        B: serde::Serialize + ?Sized,
    {
        let url = self.resolve()?;
        let bytes = bytes::Bytes::from(serde_json::to_vec(body).map_err(ZaiError::from)?);
        let request = self.prepare(
            url,
            BodyKind::Bytes(&bytes),
            ResponseMode::Audio,
            RetrySafety::for_method(self.route.method()),
        );
        self.client.inner.sender.send(&request).await?.bytes()
    }

    /// Dispatch a body-less operation and return a validated, bounded file
    /// stream.
    pub(crate) async fn send_empty_file_stream(
        self,
    ) -> ZaiResult<crate::client::transport::FileByteStream> {
        let url = self.resolve()?;
        let request = self.prepare(
            url,
            BodyKind::None,
            ResponseMode::File,
            RetrySafety::for_method(self.route.method()),
        );
        self.client.inner.sender.send_file(&request).await
    }

    /// Dispatch a replayable multipart factory and decode a JSON response.
    pub(crate) async fn send_multipart<R>(self, factory: &MultipartBodyFactory) -> ZaiResult<R>
    where
        R: serde::de::DeserializeOwned,
    {
        let url = self.resolve()?;
        let request = self.prepare(
            url,
            BodyKind::Multipart(factory),
            ResponseMode::Json,
            RetrySafety::for_method(self.route.method()),
        );
        self.client.inner.sender.send(&request).await?.json()
    }

    /// Dispatch a non-replayable JSON operation and return its SSE byte stream.
    pub(crate) async fn send_sse_json<B>(
        self,
        body: &B,
    ) -> ZaiResult<crate::client::transport::SseByteStream>
    where
        B: serde::Serialize + ?Sized,
    {
        let url = self.resolve()?;
        let bytes = bytes::Bytes::from(serde_json::to_vec(body).map_err(ZaiError::from)?);
        let request = self.prepare(
            url,
            BodyKind::Bytes(&bytes),
            ResponseMode::Json,
            RetrySafety::NonIdempotent,
        );
        self.client.inner.sender.send_sse(&request).await
    }

    /// Dispatch a non-replayable multipart operation and return its SSE stream.
    pub(crate) async fn send_sse_multipart(
        self,
        factory: &MultipartBodyFactory,
    ) -> ZaiResult<crate::client::transport::SseByteStream> {
        let url = self.resolve()?;
        let request = self.prepare(
            url,
            BodyKind::Multipart(factory),
            ResponseMode::Json,
            RetrySafety::NonIdempotent,
        );
        self.client.inner.sender.send_sse(&request).await
    }
}

impl std::fmt::Debug for Operation<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Operation")
            .field("operation_id", &self.route.operation_id())
            .field("method", &self.route.method())
            .field("route", &self.route.trace_template())
            .field("parameter_count", &self.parameters.len())
            .field("query_count", &self.query.len())
            .field("sensitive_header_count", &self.sensitive_headers.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagnostics_never_include_path_or_query_values() {
        #[derive(Serialize)]
        struct Query<'a> {
            cursor: &'a str,
        }

        let client = ZaiClient::builder("abc.0123456789abcdef").build().unwrap();
        let operation = client
            .operation(crate::client::routes::FILES_GET_CONTENT)
            .with_parameters(["private-file-id"])
            .with_query(&Query {
                cursor: "private-cursor",
            })
            .unwrap();

        let debug = format!("{operation:?}");
        assert!(debug.contains("files.content"));
        assert!(debug.contains("/files/{parameter}/content"));
        assert!(debug.contains("query_count: 1"));
        assert!(!debug.contains("private-file-id"));
        assert!(!debug.contains("private-cursor"));
    }

    #[test]
    fn operation_and_prepared_request_redact_sensitive_headers() {
        let client = ZaiClient::builder("abc.0123456789abcdef").build().unwrap();
        let secret = "private-session-123";
        let operation = client
            .operation(crate::client::routes::ZRAG_CHAT)
            .with_sensitive_header("X-Session-Id", secret)
            .unwrap();

        let operation_debug = format!("{operation:?}");
        assert!(operation_debug.contains("sensitive_header_count: 1"));
        assert!(!operation_debug.contains(secret));

        let prepared = operation.prepare(
            "https://open.bigmodel.cn/api/zrag/agent/chat".to_string(),
            BodyKind::None,
            ResponseMode::Json,
            RetrySafety::NonIdempotent,
        );
        let prepared_debug = format!("{prepared:?}");
        assert!(prepared_debug.contains("sensitive_header_count: 1"));
        assert!(!prepared_debug.contains(secret));
    }
}
