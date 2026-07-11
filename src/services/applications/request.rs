//! Request types for the 7 LLM-application endpoints (plan P06).
//!
//! Each request follows the established P05 `send_via(&ZaiClient)` pattern —
//! credentials and transport live on the client, and the request struct carries
//! only the endpoint-specific body and path parameters.
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
// transport_config helper — mirrors the pattern in every P05 service module.
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// 1. ApplicationFileStatsRequest — POST /v2/application/file_stat
// ---------------------------------------------------------------------------

/// Request body for the file-stats endpoint (open schema).
pub struct ApplicationFileStatsRequest {
    pub body: serde_json::Value,
}

impl ApplicationFileStatsRequest {
    pub fn new(body: serde_json::Value) -> Self {
        Self { body }
    }

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
    pub files: Vec<(String, Vec<u8>)>,
    pub body: serde_json::Value,
}

impl ApplicationFileUploadRequest {
    pub fn new(files: Vec<(String, Vec<u8>)>, body: serde_json::Value) -> Self {
        Self { files, body }
    }

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
    pub body: serde_json::Value,
}

impl ApplicationSliceInfoRequest {
    pub fn new(body: serde_json::Value) -> Self {
        Self { body }
    }

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
    pub app_id: String,
    pub body: serde_json::Value,
}

impl ApplicationConversationCreateRequest {
    pub fn new(app_id: impl Into<String>, body: serde_json::Value) -> Self {
        Self {
            app_id: app_id.into(),
            body,
        }
    }

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
    pub app_id: String,
}

impl ApplicationVariablesRequest {
    pub fn new(app_id: impl Into<String>) -> Self {
        Self {
            app_id: app_id.into(),
        }
    }

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
    pub app_id: String,
    pub conversation_id: String,
}

impl ApplicationHistoryRequest {
    pub fn new(app_id: impl Into<String>, conversation_id: impl Into<String>) -> Self {
        Self {
            app_id: app_id.into(),
            conversation_id: conversation_id.into(),
        }
    }

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
    pub body: serde_json::Value,
}

impl ApplicationInvokeRequest {
    pub fn new(body: serde_json::Value) -> Self {
        Self { body }
    }

    pub async fn send_via(&self, client: &ZaiClient) -> ZaiResult<ApplicationInvokeResponse> {
        let route = crate::client::routes::APPLICATIONS_INVOKE;
        let url = client.endpoints().resolve_route(route, &[])?;
        client
            .send_json::<_, ApplicationInvokeResponse>(route.method(), url, &self.body)
            .await
    }
}
