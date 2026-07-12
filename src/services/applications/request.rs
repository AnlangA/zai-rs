//! Request types for the seven LLM-application endpoints.
//!
//! Credentials and transport live on [`ZaiClient`]; each request carries only
//! its endpoint-specific body and path parameters.
//!
//! | Type | Method | Family | Path |
//! |------|--------|--------|------|
//! | [`ApplicationFileStatsRequest`] | POST | `ApplicationV2` | `v2/application/file_stat` |
//! | [`ApplicationFileUploadRequest`] | POST multipart | `ApplicationV2` | `v2/application/file_upload` |
//! | [`ApplicationSliceInfoRequest`] | POST | `ApplicationV2` | `v2/application/slice_info` |
//! | [`ApplicationConversationCreateRequest`] | POST | `ApplicationV2` | `v2/application/{app_id}/conversation` |
//! | [`ApplicationVariablesRequest`] | GET | `ApplicationV2` | `v2/application/{app_id}/variables` |
//! | [`ApplicationHistoryRequest`] | GET | `LlmApplication` | `history_session_record/{app_id}/{conversation_id}` |
//! | [`ApplicationInvokeRequest`] | POST | `ApplicationV3` | `v3/application/invoke` |

use crate::ZaiResult;
use crate::client::ZaiClient;
use crate::services::applications::response::{
    ApplicationConversationCreateResponse, ApplicationFileStatsResponse,
    ApplicationFileUploadResponse, ApplicationHistoryResponse, ApplicationInvokeResponse,
    ApplicationSliceInfoResponse, ApplicationVariablesResponse,
};

// ---------------------------------------------------------------------------
// 1. ApplicationFileStatsRequest — POST /v2/application/file_stat
// ---------------------------------------------------------------------------

/// Request body for the file-stats endpoint (open schema).
pub struct ApplicationFileStatsRequest {
    /// Open-schema request body sent as JSON.
    pub body: serde_json::Value,
}

impl ApplicationFileStatsRequest {
    /// Create a file-statistics request from an open-schema JSON body.
    pub fn new(body: serde_json::Value) -> Self {
        Self { body }
    }

    /// Send the request through `client`.
    pub async fn send_via(&self, client: &ZaiClient) -> ZaiResult<ApplicationFileStatsResponse> {
        let route = crate::client::routes::APPLICATIONS_FILE_STATS;
        let url = client.endpoints().resolve_route(route, &[])?;
        client
            .send_json::<_, ApplicationFileStatsResponse>(route.method(), url, &self.body)
            .await
    }
}

// ---------------------------------------------------------------------------
// 2. ApplicationFileUploadRequest — POST multipart /v2/application/file_upload
// ---------------------------------------------------------------------------

/// File upload request (multipart/form-data). The `files` vector holds
/// (filename, bytes) pairs; `body` carries additional form fields as a JSON
/// value whose top-level entries are serialized as text form fields.
pub struct ApplicationFileUploadRequest {
    /// Files represented as `(filename, bytes)` pairs.
    pub files: Vec<(String, Vec<u8>)>,
    /// Additional multipart text fields represented by a JSON object.
    pub body: serde_json::Value,
}

impl ApplicationFileUploadRequest {
    /// Create a multipart upload request.
    ///
    /// When `body` is not a JSON object it contributes no multipart fields.
    pub fn new(files: Vec<(String, Vec<u8>)>, body: serde_json::Value) -> Self {
        Self { files, body }
    }

    /// Send the multipart request through `client`.
    pub async fn send_via(&self, client: &ZaiClient) -> ZaiResult<ApplicationFileUploadResponse> {
        let route = crate::client::routes::APPLICATIONS_UPLOAD_FILE;
        let url = client.endpoints().resolve_route(route, &[])?;
        let mut factory = crate::client::transport::multipart::MultipartBodyFactory::new();
        if let serde_json::Value::Object(map) = &self.body {
            for (key, value) in map {
                let text = match value {
                    serde_json::Value::String(s) => s.clone(),
                    other => other.to_string(),
                };
                factory = factory.field(key.clone(), text)?;
            }
        }
        for (filename, bytes) in &self.files {
            factory = factory.bytes_named(
                "files",
                filename.clone(),
                "application/octet-stream",
                bytes.clone(),
            )?;
        }
        client
            .send_multipart::<ApplicationFileUploadResponse>(route.method(), url, &factory)
            .await
    }
}

// ---------------------------------------------------------------------------
// 3. ApplicationSliceInfoRequest — POST /v2/application/slice_info
// ---------------------------------------------------------------------------

/// Request body for the slice-info endpoint (open schema).
pub struct ApplicationSliceInfoRequest {
    /// Open-schema request body sent as JSON.
    pub body: serde_json::Value,
}

impl ApplicationSliceInfoRequest {
    /// Create a slice-information request from an open-schema JSON body.
    pub fn new(body: serde_json::Value) -> Self {
        Self { body }
    }

    /// Send the request through `client`.
    pub async fn send_via(&self, client: &ZaiClient) -> ZaiResult<ApplicationSliceInfoResponse> {
        let route = crate::client::routes::APPLICATIONS_SLICE_INFO;
        let url = client.endpoints().resolve_route(route, &[])?;
        client
            .send_json::<_, ApplicationSliceInfoResponse>(route.method(), url, &self.body)
            .await
    }
}

// ---------------------------------------------------------------------------
// 4. ApplicationConversationCreateRequest — POST /v2/application/{app_id}/conversation
// ---------------------------------------------------------------------------

/// Create a conversation under an application. `app_id` is a dynamic path
/// segment; `body` carries the open-schema request payload.
pub struct ApplicationConversationCreateRequest {
    /// Application identifier inserted into the request path.
    pub app_id: String,
    /// Open-schema conversation body sent as JSON.
    pub body: serde_json::Value,
}

impl ApplicationConversationCreateRequest {
    /// Create a conversation request for an application.
    pub fn new(app_id: impl Into<String>, body: serde_json::Value) -> Self {
        Self {
            app_id: app_id.into(),
            body,
        }
    }

    /// Send the request through `client`.
    pub async fn send_via(
        &self,
        client: &ZaiClient,
    ) -> ZaiResult<ApplicationConversationCreateResponse> {
        let route = crate::client::routes::APPLICATIONS_CREATE_CONVERSATION;
        let url = client.endpoints().resolve_route(route, &[&self.app_id])?;
        client
            .send_json::<_, ApplicationConversationCreateResponse>(route.method(), url, &self.body)
            .await
    }
}

// ---------------------------------------------------------------------------
// 5. ApplicationVariablesRequest — GET /v2/application/{app_id}/variables
// ---------------------------------------------------------------------------

/// Retrieve variables for an application. No request body; `app_id` is the
/// sole path parameter.
pub struct ApplicationVariablesRequest {
    /// Application identifier inserted into the request path.
    pub app_id: String,
}

impl ApplicationVariablesRequest {
    /// Create a request for one application's variables.
    pub fn new(app_id: impl Into<String>) -> Self {
        Self {
            app_id: app_id.into(),
        }
    }

    /// Send the request through `client`.
    pub async fn send_via(&self, client: &ZaiClient) -> ZaiResult<ApplicationVariablesResponse> {
        let route = crate::client::routes::APPLICATIONS_VARIABLES;
        let url = client.endpoints().resolve_route(route, &[&self.app_id])?;
        client
            .send_empty::<ApplicationVariablesResponse>(route.method(), url)
            .await
    }
}

// ---------------------------------------------------------------------------
// 6. ApplicationHistoryRequest — GET /history_session_record/{app_id}/{conversation_id}
// ---------------------------------------------------------------------------

/// Retrieve conversation history. Uses the `LlmApplication` family (no
/// version-prefix in the path).
pub struct ApplicationHistoryRequest {
    /// Application identifier inserted into the request path.
    pub app_id: String,
    /// Conversation identifier inserted into the request path.
    pub conversation_id: String,
}

impl ApplicationHistoryRequest {
    /// Create a conversation-history request.
    pub fn new(app_id: impl Into<String>, conversation_id: impl Into<String>) -> Self {
        Self {
            app_id: app_id.into(),
            conversation_id: conversation_id.into(),
        }
    }

    /// Send the request through `client`.
    pub async fn send_via(&self, client: &ZaiClient) -> ZaiResult<ApplicationHistoryResponse> {
        let route = crate::client::routes::APPLICATIONS_HISTORY;
        let url = client
            .endpoints()
            .resolve_route(route, &[&self.app_id, &self.conversation_id])?;
        client
            .send_empty::<ApplicationHistoryResponse>(route.method(), url)
            .await
    }
}

// ---------------------------------------------------------------------------
// 7. ApplicationInvokeRequest — POST /v3/application/invoke
// ---------------------------------------------------------------------------

/// Invoke an LLM application (V3 family). The `body` carries the open-schema
/// invoke payload.
pub struct ApplicationInvokeRequest {
    /// Open-schema invocation body sent as JSON.
    pub body: serde_json::Value,
}

impl ApplicationInvokeRequest {
    /// Create an application invocation request.
    pub fn new(body: serde_json::Value) -> Self {
        Self { body }
    }

    /// Send the request through `client`.
    pub async fn send_via(&self, client: &ZaiClient) -> ZaiResult<ApplicationInvokeResponse> {
        let route = crate::client::routes::APPLICATIONS_INVOKE;
        let url = client.endpoints().resolve_route(route, &[])?;
        client
            .send_json::<_, ApplicationInvokeResponse>(route.method(), url, &self.body)
            .await
    }
}
