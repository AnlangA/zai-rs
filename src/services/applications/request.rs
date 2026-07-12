//! Typed requests for the seven LLM-application endpoints.
//!
//! Credentials and transport live on [`ZaiClient`]; each request models only
//! the fields declared by the frozen OpenAPI operation.

use serde::Serialize;

use crate::services::applications::response::{
    ApplicationConversationCreateResponse, ApplicationFileStatsResponse,
    ApplicationFileUploadResponse, ApplicationHistoryResponse, ApplicationInvokeResponse,
    ApplicationSliceInfoResponse, ApplicationVariablesResponse,
};
use crate::{ZaiResult, client::ZaiClient};

/// Request body for application file parsing statuses.
#[derive(Clone, PartialEq, Eq, Serialize)]
pub struct ApplicationFileStatsRequest {
    /// Application identifier.
    pub app_id: String,
    /// File identifiers whose parsing status should be returned.
    pub file_ids: Vec<String>,
}

impl std::fmt::Debug for ApplicationFileStatsRequest {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ApplicationFileStatsRequest")
            .field("app_id", &"[REDACTED]")
            .field("file_count", &self.file_ids.len())
            .finish()
    }
}

impl ApplicationFileStatsRequest {
    /// Create a file-statistics request with all OpenAPI-required fields.
    pub fn new(app_id: impl Into<String>, file_ids: Vec<String>) -> Self {
        Self {
            app_id: app_id.into(),
            file_ids,
        }
    }

    /// Validate the required application id and non-empty file-id list.
    pub fn validate(&self) -> ZaiResult<()> {
        crate::client::validation::require_non_blank(&self.app_id, "app_id")?;
        if self.file_ids.is_empty() {
            return Err(crate::client::validation::invalid(
                "at least one file_id is required",
            ));
        }
        for file_id in &self.file_ids {
            crate::client::validation::require_non_blank(file_id, "file_id")?;
        }
        Ok(())
    }

    /// Send the request through `client`.
    pub async fn send_via(&self, client: &ZaiClient) -> ZaiResult<ApplicationFileStatsResponse> {
        self.validate()?;
        let route = crate::client::routes::APPLICATIONS_FILE_STATS;
        let url = client.endpoints().resolve_route(route, &[])?;
        client
            .send_json::<_, ApplicationFileStatsResponse>(route.method(), url, self)
            .await
    }
}

/// Multipart request for uploading one or more application files.
pub struct ApplicationFileUploadRequest {
    /// Application identifier (required multipart field `app_id`).
    pub app_id: String,
    /// Files represented as `(filename, bytes)` pairs. Each file is encoded as
    /// a separate multipart field named `files`.
    pub files: Vec<(String, Vec<u8>)>,
    /// Upload component identifier for text applications.
    pub upload_unit_id: Option<String>,
    /// Conversation identifier for temporary conversation files.
    pub conversation_id: Option<String>,
    /// Upstream numeric file type (`1` through `5` are currently documented).
    pub file_type: Option<i64>,
}

impl std::fmt::Debug for ApplicationFileUploadRequest {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ApplicationFileUploadRequest")
            .field("app_id", &"[REDACTED]")
            .field("file_count", &self.files.len())
            .field("upload_unit_id_configured", &self.upload_unit_id.is_some())
            .field(
                "conversation_id_configured",
                &self.conversation_id.is_some(),
            )
            .field("file_type", &self.file_type)
            .finish()
    }
}

impl ApplicationFileUploadRequest {
    /// Create an upload request with the required application and files.
    pub fn new(app_id: impl Into<String>, files: Vec<(String, Vec<u8>)>) -> Self {
        Self {
            app_id: app_id.into(),
            files,
            upload_unit_id: None,
            conversation_id: None,
            file_type: None,
        }
    }

    /// Set the upload component identifier.
    pub fn with_upload_unit_id(mut self, upload_unit_id: impl Into<String>) -> Self {
        self.upload_unit_id = Some(upload_unit_id.into());
        self
    }

    /// Set the conversation identifier for temporary files.
    pub fn with_conversation_id(mut self, conversation_id: impl Into<String>) -> Self {
        self.conversation_id = Some(conversation_id.into());
        self
    }

    /// Set the upstream numeric file type.
    pub fn with_file_type(mut self, file_type: i64) -> Self {
        self.file_type = Some(file_type);
        self
    }

    /// Validate the required repeated binary field before allocating a body.
    pub fn validate(&self) -> ZaiResult<()> {
        crate::client::validation::require_non_blank(&self.app_id, "app_id")?;
        if self.files.is_empty() {
            return Err(crate::client::validation::invalid(
                "at least one file is required",
            ));
        }
        if let Some((index, _)) = self
            .files
            .iter()
            .enumerate()
            .find(|(_, (filename, _))| filename.trim().is_empty())
        {
            return Err(crate::client::validation::invalid(format!(
                "file at index {index} must have a non-blank filename"
            )));
        }
        if let Some(upload_unit_id) = self.upload_unit_id.as_deref() {
            crate::client::validation::require_non_blank(upload_unit_id, "upload_unit_id")?;
        }
        if let Some(conversation_id) = self.conversation_id.as_deref() {
            crate::client::validation::require_non_blank(conversation_id, "conversation_id")?;
        }
        if self
            .file_type
            .is_some_and(|file_type| !(1..=5).contains(&file_type))
        {
            return Err(crate::client::validation::invalid(
                "file_type must be between 1 and 5",
            ));
        }
        Ok(())
    }

    /// Send the multipart request through `client`.
    pub async fn send_via(&self, client: &ZaiClient) -> ZaiResult<ApplicationFileUploadResponse> {
        self.validate()?;
        let route = crate::client::routes::APPLICATIONS_UPLOAD_FILE;
        let url = client.endpoints().resolve_route(route, &[])?;
        let mut factory = crate::client::transport::multipart::MultipartBodyFactory::new()
            .field("app_id", self.app_id.clone())?;

        if let Some(upload_unit_id) = &self.upload_unit_id {
            factory = factory.field("upload_unit_id", upload_unit_id.clone())?;
        }
        if let Some(conversation_id) = &self.conversation_id {
            factory = factory.field("conversation_id", conversation_id.clone())?;
        }
        if let Some(file_type) = self.file_type {
            factory = factory.field("file_type", file_type.to_string())?;
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

/// Request body for application document slice information.
#[derive(Clone, PartialEq, Eq, Serialize)]
pub struct ApplicationSliceInfoRequest {
    /// Identifier returned by conversation creation or text invocation.
    pub request_id: String,
    /// Application node identifier.
    pub node_id: String,
}

impl std::fmt::Debug for ApplicationSliceInfoRequest {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ApplicationSliceInfoRequest")
            .field("request_id", &"[REDACTED]")
            .field("node_id", &"[REDACTED]")
            .finish()
    }
}

impl ApplicationSliceInfoRequest {
    /// Create a slice-information request with both required identifiers.
    pub fn new(request_id: impl Into<String>, node_id: impl Into<String>) -> Self {
        Self {
            request_id: request_id.into(),
            node_id: node_id.into(),
        }
    }

    /// Validate both required identifiers.
    pub fn validate(&self) -> ZaiResult<()> {
        crate::client::validation::require_non_blank(&self.request_id, "request_id")?;
        crate::client::validation::require_non_blank(&self.node_id, "node_id")
    }

    /// Send the request through `client`.
    pub async fn send_via(&self, client: &ZaiClient) -> ZaiResult<ApplicationSliceInfoResponse> {
        self.validate()?;
        let route = crate::client::routes::APPLICATIONS_SLICE_INFO;
        let url = client.endpoints().resolve_route(route, &[])?;
        client
            .send_json::<_, ApplicationSliceInfoResponse>(route.method(), url, self)
            .await
    }
}

/// Create a conversation under an application.
///
/// The frozen operation has no request body; `app_id` is its sole input.
#[derive(Clone, PartialEq, Eq)]
pub struct ApplicationConversationCreateRequest {
    /// Application identifier inserted into the request path.
    pub app_id: String,
}

impl std::fmt::Debug for ApplicationConversationCreateRequest {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ApplicationConversationCreateRequest")
            .field("app_id", &"[REDACTED]")
            .finish()
    }
}

impl ApplicationConversationCreateRequest {
    /// Create a conversation request for an application.
    pub fn new(app_id: impl Into<String>) -> Self {
        Self {
            app_id: app_id.into(),
        }
    }

    /// Validate the dynamic path identifier.
    pub fn validate(&self) -> ZaiResult<()> {
        crate::client::validation::require_non_blank(&self.app_id, "app_id")
    }

    /// Send the bodyless request through `client`.
    pub async fn send_via(
        &self,
        client: &ZaiClient,
    ) -> ZaiResult<ApplicationConversationCreateResponse> {
        self.validate()?;
        let route = crate::client::routes::APPLICATIONS_CREATE_CONVERSATION;
        let url = client.endpoints().resolve_route(route, &[&self.app_id])?;
        client
            .send_empty::<ApplicationConversationCreateResponse>(route.method(), url)
            .await
    }
}

/// Retrieve variables for an application.
#[derive(Clone, PartialEq, Eq)]
pub struct ApplicationVariablesRequest {
    /// Application identifier inserted into the request path.
    pub app_id: String,
}

impl std::fmt::Debug for ApplicationVariablesRequest {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ApplicationVariablesRequest")
            .field("app_id", &"[REDACTED]")
            .finish()
    }
}

impl ApplicationVariablesRequest {
    /// Create a request for one application's variables.
    pub fn new(app_id: impl Into<String>) -> Self {
        Self {
            app_id: app_id.into(),
        }
    }

    /// Validate the dynamic path identifier.
    pub fn validate(&self) -> ZaiResult<()> {
        crate::client::validation::require_non_blank(&self.app_id, "app_id")
    }

    /// Send the request through `client`.
    pub async fn send_via(&self, client: &ZaiClient) -> ZaiResult<ApplicationVariablesResponse> {
        self.validate()?;
        let route = crate::client::routes::APPLICATIONS_VARIABLES;
        let url = client.endpoints().resolve_route(route, &[&self.app_id])?;
        client
            .send_empty::<ApplicationVariablesResponse>(route.method(), url)
            .await
    }
}

/// Retrieve recommended questions for an application conversation.
#[derive(Clone, PartialEq, Eq)]
pub struct ApplicationHistoryRequest {
    /// Application identifier inserted into the request path.
    pub app_id: String,
    /// Conversation identifier inserted into the request path.
    pub conversation_id: String,
}

impl std::fmt::Debug for ApplicationHistoryRequest {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ApplicationHistoryRequest")
            .field("app_id", &"[REDACTED]")
            .field("conversation_id", &"[REDACTED]")
            .finish()
    }
}

impl ApplicationHistoryRequest {
    /// Create a conversation-history request.
    pub fn new(app_id: impl Into<String>, conversation_id: impl Into<String>) -> Self {
        Self {
            app_id: app_id.into(),
            conversation_id: conversation_id.into(),
        }
    }

    /// Validate both dynamic path identifiers.
    pub fn validate(&self) -> ZaiResult<()> {
        crate::client::validation::require_non_blank(&self.app_id, "app_id")?;
        crate::client::validation::require_non_blank(&self.conversation_id, "conversation_id")
    }

    /// Send the request through `client`.
    pub async fn send_via(&self, client: &ZaiClient) -> ZaiResult<ApplicationHistoryResponse> {
        self.validate()?;
        let route = crate::client::routes::APPLICATIONS_HISTORY;
        let url = client
            .endpoints()
            .resolve_route(route, &[&self.app_id, &self.conversation_id])?;
        client
            .send_empty::<ApplicationHistoryResponse>(route.method(), url)
            .await
    }
}

/// One typed content value in an application invocation message.
#[derive(Clone, PartialEq, Eq, Serialize)]
pub struct ApplicationInvokeContent {
    /// Upstream content type, for example `input` or `upload_file`.
    #[serde(rename = "type")]
    pub type_: String,
    /// Text, selection, file identifier, or media URL.
    pub value: String,
    /// Field name, required by the service for text applications.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
}

impl std::fmt::Debug for ApplicationInvokeContent {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ApplicationInvokeContent")
            .field("type", &"[REDACTED]")
            .field("value", &"[REDACTED]")
            .field("key_configured", &self.key.is_some())
            .finish()
    }
}

impl ApplicationInvokeContent {
    /// Create a content value with both OpenAPI-required fields.
    pub fn new(type_: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            type_: type_.into(),
            value: value.into(),
            key: None,
        }
    }

    /// Set the text-application field name.
    pub fn with_key(mut self, key: impl Into<String>) -> Self {
        self.key = Some(key.into());
        self
    }

    fn validate(&self) -> ZaiResult<()> {
        crate::client::validation::require_non_blank(&self.type_, "content.type")?;
        crate::client::validation::require_non_blank(&self.value, "content.value")?;
        if let Some(key) = self.key.as_deref() {
            crate::client::validation::require_non_blank(key, "content.key")?;
        }
        Ok(())
    }
}

/// One message sent to an application invocation.
#[derive(Clone, PartialEq, Eq, Serialize)]
pub struct ApplicationInvokeMessage {
    /// Optional role supplied for conversational applications.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    /// Typed content values. The field itself is required by OpenAPI.
    pub content: Vec<ApplicationInvokeContent>,
}

impl std::fmt::Debug for ApplicationInvokeMessage {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ApplicationInvokeMessage")
            .field("role_configured", &self.role.is_some())
            .field("content_count", &self.content.len())
            .finish()
    }
}

impl ApplicationInvokeMessage {
    /// Create a message from its required content array.
    pub fn new(content: Vec<ApplicationInvokeContent>) -> Self {
        Self {
            role: None,
            content,
        }
    }

    /// Set the optional message role.
    pub fn with_role(mut self, role: impl Into<String>) -> Self {
        self.role = Some(role.into());
        self
    }

    fn validate(&self) -> ZaiResult<()> {
        if self.content.is_empty() {
            return Err(crate::client::validation::invalid(
                "application message content must not be empty",
            ));
        }
        if let Some(role) = self.role.as_deref() {
            crate::client::validation::require_non_blank(role, "message.role")?;
        }
        self.content
            .iter()
            .try_for_each(ApplicationInvokeContent::validate)
    }
}

/// Request body for an application-v3 invocation.
#[derive(Clone, PartialEq, Eq, Serialize)]
pub struct ApplicationInvokeRequest {
    /// Application identifier.
    pub app_id: String,
    /// Conversation identifier; omitted to create a new conversation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conversation_id: Option<String>,
    /// Caller-provided tracing identifier for plugin invocations.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub third_request_id: Option<String>,
    /// Streaming flag. The JSON-only `send_via` path requires this to remain
    /// `false` and serializes it explicitly because the server default is true.
    #[serde(default)]
    pub stream: bool,
    /// Invocation messages.
    pub messages: Vec<ApplicationInvokeMessage>,
    /// Top-level role required by some conversational applications.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    /// Whether process-log events should be emitted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub send_log_event: Option<bool>,
}

impl std::fmt::Debug for ApplicationInvokeRequest {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ApplicationInvokeRequest")
            .field("app_id", &"[REDACTED]")
            .field(
                "conversation_id_configured",
                &self.conversation_id.is_some(),
            )
            .field(
                "third_request_id_configured",
                &self.third_request_id.is_some(),
            )
            .field("stream", &self.stream)
            .field("message_count", &self.messages.len())
            .field("role_configured", &self.role.is_some())
            .field("send_log_event", &self.send_log_event)
            .finish()
    }
}

impl ApplicationInvokeRequest {
    /// Create an invocation with all OpenAPI-required fields.
    pub fn new(app_id: impl Into<String>, messages: Vec<ApplicationInvokeMessage>) -> Self {
        Self {
            app_id: app_id.into(),
            conversation_id: None,
            third_request_id: None,
            stream: false,
            messages,
            role: None,
            send_log_event: None,
        }
    }

    /// Continue an existing conversation.
    pub fn with_conversation_id(mut self, conversation_id: impl Into<String>) -> Self {
        self.conversation_id = Some(conversation_id.into());
        self
    }

    /// Set a third-party tracing identifier.
    pub fn with_third_request_id(mut self, third_request_id: impl Into<String>) -> Self {
        self.third_request_id = Some(third_request_id.into());
        self
    }

    /// Set the top-level conversational role.
    pub fn with_role(mut self, role: impl Into<String>) -> Self {
        self.role = Some(role.into());
        self
    }

    /// Configure process-log event delivery.
    pub fn with_log_events(mut self, enabled: bool) -> Self {
        self.send_log_event = Some(enabled);
        self
    }

    /// Validate the JSON-only operation invariant.
    pub fn validate(&self) -> ZaiResult<()> {
        crate::client::validation::require_non_blank(&self.app_id, "app_id")?;
        if self.stream {
            return Err(crate::client::validation::invalid(
                "application send_via supports only stream=false",
            ));
        }
        if self.messages.is_empty() {
            return Err(crate::client::validation::invalid(
                "application messages must not be empty",
            ));
        }
        for (value, name) in [
            (self.conversation_id.as_deref(), "conversation_id"),
            (self.third_request_id.as_deref(), "third_request_id"),
            (self.role.as_deref(), "role"),
        ] {
            if let Some(value) = value {
                crate::client::validation::require_non_blank(value, name)?;
            }
        }
        self.messages
            .iter()
            .try_for_each(ApplicationInvokeMessage::validate)?;
        Ok(())
    }

    /// Send the request through `client`.
    pub async fn send_via(&self, client: &ZaiClient) -> ZaiResult<ApplicationInvokeResponse> {
        self.validate()?;
        let route = crate::client::routes::APPLICATIONS_INVOKE;
        let url = client.endpoints().resolve_route(route, &[])?;
        let response = client
            .send_json::<_, ApplicationInvokeResponse>(route.method(), url, self)
            .await?;
        response.validate()?;
        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closed_json_requests_serialize_required_fields_exactly() {
        assert_eq!(
            serde_json::to_value(ApplicationFileStatsRequest::new(
                "app-1",
                vec!["file-1".to_owned()]
            ))
            .unwrap(),
            serde_json::json!({"app_id": "app-1", "file_ids": ["file-1"]})
        );
        assert_eq!(
            serde_json::to_value(ApplicationSliceInfoRequest::new("req-1", "node-1")).unwrap(),
            serde_json::json!({"request_id": "req-1", "node_id": "node-1"})
        );

        let invoke = ApplicationInvokeRequest::new(
            "app-1",
            vec![
                ApplicationInvokeMessage::new(vec![
                    ApplicationInvokeContent::new("input", "hello").with_key("question"),
                ])
                .with_role("user"),
            ],
        );
        assert_eq!(
            serde_json::to_value(invoke).unwrap(),
            serde_json::json!({
                "app_id": "app-1",
                "stream": false,
                "messages": [{
                    "role": "user",
                    "content": [{"type": "input", "value": "hello", "key": "question"}]
                }]
            })
        );
    }

    #[test]
    fn upload_validation_rejects_a_missing_or_unnamed_file() {
        assert!(
            ApplicationFileUploadRequest::new("app-1", Vec::new())
                .validate()
                .is_err()
        );
        assert!(
            ApplicationFileUploadRequest::new("app-1", vec![(String::new(), vec![1])])
                .validate()
                .is_err()
        );
        assert!(
            ApplicationFileUploadRequest::new(" ", vec![("file.txt".into(), vec![1])])
                .validate()
                .is_err()
        );
        assert!(
            ApplicationFileUploadRequest::new("app-1", vec![("file.txt".into(), vec![1])])
                .with_file_type(6)
                .validate()
                .is_err()
        );
    }

    #[test]
    fn path_requests_reject_blank_identifiers() {
        assert!(
            ApplicationConversationCreateRequest::new(" ")
                .validate()
                .is_err()
        );
        assert!(ApplicationVariablesRequest::new("").validate().is_err());
        assert!(
            ApplicationHistoryRequest::new("app", " ")
                .validate()
                .is_err()
        );
    }

    #[test]
    fn json_invoke_rejects_the_unsupported_streaming_mode() {
        let mut request = ApplicationInvokeRequest::new(
            "app-1",
            vec![ApplicationInvokeMessage::new(vec![
                ApplicationInvokeContent::new("input", "hello"),
            ])],
        );
        request.stream = true;
        assert!(request.validate().is_err());
    }

    #[test]
    fn required_json_fields_are_non_blank_and_non_empty() {
        assert!(
            ApplicationFileStatsRequest::new(" ", vec!["file".into()])
                .validate()
                .is_err()
        );
        assert!(
            ApplicationFileStatsRequest::new("app", Vec::new())
                .validate()
                .is_err()
        );
        assert!(
            ApplicationSliceInfoRequest::new("request", " ")
                .validate()
                .is_err()
        );
        assert!(
            ApplicationInvokeRequest::new("app", Vec::new())
                .validate()
                .is_err()
        );
        assert!(
            ApplicationInvokeRequest::new(
                "app",
                vec![ApplicationInvokeMessage::new(vec![
                    ApplicationInvokeContent::new("input", " "),
                ])],
            )
            .validate()
            .is_err()
        );
    }

    #[test]
    fn debug_output_redacts_application_content_and_identifiers() {
        let stats =
            ApplicationFileStatsRequest::new("private-app", vec!["private-file".to_owned()]);
        let invoke = ApplicationInvokeRequest::new(
            "private-app",
            vec![
                ApplicationInvokeMessage::new(vec![
                    ApplicationInvokeContent::new("private-type", "private-value")
                        .with_key("private-key"),
                ])
                .with_role("private-role"),
            ],
        )
        .with_conversation_id("private-conversation")
        .with_third_request_id("private-request")
        .with_role("private-top-role");
        let debug = format!("{stats:?} {invoke:?}");
        for secret in [
            "private-app",
            "private-file",
            "private-type",
            "private-value",
            "private-key",
            "private-role",
            "private-conversation",
            "private-request",
            "private-top-role",
        ] {
            assert!(!debug.contains(secret), "Debug leaked {secret}");
        }
    }
}
