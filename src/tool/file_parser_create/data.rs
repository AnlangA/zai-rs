//! # File Parser Creation API
//!
//! This module provides the file parser creation client for creating file
//! parsing tasks.

use std::{path::Path, sync::Arc};

use tracing::{debug, trace, warn};

use super::{request::*, response::*};
use crate::{
    ZaiResult,
    client::{
        endpoints::{ApiBase, EndpointConfig, paths},
        error::codes,
        http::{HttpClientConfig, parse_typed_response, send_multipart_request},
    },
};

/// File parser creation client.
///
/// This client provides functionality to create file parsing tasks,
/// supporting multiple file formats and parsing tools.
///
/// ## Examples
///
/// ```rust,ignore
/// use zai_rs::tool::file_parser_create::{FileParserCreateRequest, ToolType, FileType};
/// use std::path::Path;
///
/// let api_key = "your-api-key".to_string();
/// let file_path = Path::new("document.pdf");
///
/// let request = FileParserCreateRequest::new(
///     api_key,
///     file_path,
///     ToolType::Lite,
///     FileType::PDF,
/// )?;
/// ```
pub struct FileParserCreateRequest {
    /// API key for authentication
    pub key: String,
    url: String,
    endpoint_config: EndpointConfig,
    api_base: ApiBase,
    http_config: Arc<HttpClientConfig>,
    /// Path to the file to parse
    pub file_path: std::path::PathBuf,
    /// Parsing tool type to use
    pub tool_type: ToolType,
    /// File type to parse
    pub file_type: FileType,
}

impl FileParserCreateRequest {
    /// Creates a new file parser creation request.
    ///
    /// ## Arguments
    ///
    /// * `key` - API key for authentication
    /// * `file_path` - Path to the file to parse
    /// * `tool_type` - Type of parsing tool to use
    /// * `file_type` - Type of file to parse
    ///
    /// ## Returns
    ///
    /// A new `FileParserCreateRequest` instance or an error if validation
    /// fails.
    pub fn new(
        key: impl Into<String>,
        file_path: &Path,
        tool_type: ToolType,
        file_type: FileType,
    ) -> crate::ZaiResult<Self> {
        // Validate that file exists
        if !file_path.exists() {
            return Err(crate::client::error::ZaiError::FileError {
                code: codes::SDK_FILE_NOT_FOUND,
                message: format!("File does not exist: {}", file_path.display()),
            });
        }

        // Validate that file type is supported by tool
        if !file_type.is_supported_by(&tool_type) {
            return Err(crate::client::error::ZaiError::ApiError {
                code: crate::client::error::codes::SDK_VALIDATION,
                message: format!(
                    "File type {file_type:?} is not supported by tool type {tool_type:?}"
                ),
            });
        }

        let endpoint_config = EndpointConfig::default();
        let api_base = ApiBase::PaasV4;
        let url = endpoint_config.url(&api_base, paths::FILE_PARSER_CREATE);

        Ok(Self {
            key: key.into(),
            url,
            endpoint_config,
            api_base,
            http_config: Arc::new(HttpClientConfig::default()),
            file_path: file_path.to_path_buf(),
            tool_type,
            file_type,
        })
    }

    /// Creates a new file parser creation request with automatic file type
    /// detection.
    ///
    /// ## Arguments
    ///
    /// * `key` - API key for authentication
    /// * `file_path` - Path to the file to parse
    /// * `tool_type` - Type of parsing tool to use
    ///
    /// ## Returns
    ///
    /// A new `FileParserCreateRequest` instance or an error if validation
    /// fails.
    pub fn new_with_auto_type(
        key: impl Into<String>,
        file_path: &Path,
        tool_type: ToolType,
    ) -> crate::ZaiResult<Self> {
        let file_type = FileType::from_path(file_path).ok_or_else(|| {
            crate::client::error::ZaiError::FileError {
                code: codes::SDK_FILE_TYPE_UNSUPPORTED,
                message: format!(
                    "Could not determine file type from path: {}",
                    file_path.display()
                ),
            }
        })?;

        Self::new(key, file_path, tool_type, file_type)
    }

    /// Override the base URL (uses [`ApiBase::Custom`]).
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.api_base = ApiBase::Custom(base_url.into());
        self.url = self
            .endpoint_config
            .url(&self.api_base, paths::FILE_PARSER_CREATE);
        self
    }

    /// Replace the full [`EndpointConfig`] used to resolve URLs.
    pub fn with_endpoint_config(mut self, endpoint_config: EndpointConfig) -> Self {
        self.endpoint_config = endpoint_config;
        self.url = self
            .endpoint_config
            .url(&self.api_base, paths::FILE_PARSER_CREATE);
        self
    }

    /// Replace the HTTP client configuration (timeouts, retries, …).
    pub fn with_http_config(mut self, config: HttpClientConfig) -> Self {
        self.http_config = Arc::new(config);
        self
    }

    /// Sends the file parser task creation request.
    ///
    /// ## Returns
    ///
    /// A `FileParserCreateResponse` containing the task ID and status.
    pub async fn send(&self) -> ZaiResult<FileParserCreateResponse> {
        debug!(file = %self.file_path.display(), "Creating file parser task");

        let file_bytes = tokio::fs::read(&self.file_path).await?;
        let file_name = self
            .file_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        trace!(bytes = file_bytes.len(), file_name = %file_name, "Prepared parser upload");

        let url = self.url.clone();
        let key = self.key.clone();
        let config = self.http_config.clone();
        let tool_type = self.tool_type.clone();
        let file_type = self.file_type.clone();
        let response = send_multipart_request(reqwest::Method::POST, url, key, config, move || {
            let file_part = reqwest::multipart::Part::bytes(file_bytes.clone())
                .file_name(file_name.clone())
                .mime_str("application/octet-stream")?;
            Ok(reqwest::multipart::Form::new()
                .part("file", file_part)
                .text("tool_type", format!("{tool_type:?}").to_lowercase())
                .text("file_type", format!("{file_type:?}")))
        })
        .await
        .map_err(|e| e.context("file parser create"))?;

        let create_response = parse_typed_response::<FileParserCreateResponse>(response)
            .await
            .map_err(|e| e.context("file parser create"))?;

        debug!(task_id = %create_response.task_id, "File parser task created");

        if !create_response.is_success() {
            warn!(
                message = %create_response.message,
                "File parser task creation rejected by server"
            );
            return Err(crate::client::error::ZaiError::ApiError {
                code: codes::SDK_EXTERNAL_TOOL,
                message: format!("Task creation failed: {}", create_response.message),
            });
        }

        Ok(create_response)
    }
}
