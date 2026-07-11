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
        error::codes,
        http::{HttpClientConfig, parse_typed_response, send_multipart_request},
        {ApiFamily, ZaiClient},
    },
};

/// File parser creation client (P05: routes through [`ZaiClient`]).
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
/// let file_path = Path::new("document.pdf");
///
/// let request = FileParserCreateRequest::new(
///     file_path,
///     ToolType::Lite,
///     FileType::PDF,
/// )?;
/// ```
pub struct FileParserCreateRequest {
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
    /// * `file_path` - Path to the file to parse
    /// * `tool_type` - Type of parsing tool to use
    /// * `file_type` - Type of file to parse
    ///
    /// ## Returns
    ///
    /// A new `FileParserCreateRequest` instance or an error if validation
    /// fails.
    pub fn new(
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

        Ok(Self {
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
    /// * `file_path` - Path to the file to parse
    /// * `tool_type` - Type of parsing tool to use
    ///
    /// ## Returns
    ///
    /// A new `FileParserCreateRequest` instance or an error if validation
    /// fails.
    pub fn new_with_auto_type(file_path: &Path, tool_type: ToolType) -> crate::ZaiResult<Self> {
        let file_type = FileType::from_path(file_path).ok_or_else(|| {
            crate::client::error::ZaiError::FileError {
                code: codes::SDK_FILE_TYPE_UNSUPPORTED,
                message: format!(
                    "Could not determine file type from path: {}",
                    file_path.display()
                ),
            }
        })?;

        Self::new(file_path, tool_type, file_type)
    }

    /// Sends the file parser task creation request via a [`ZaiClient`].
    ///
    /// ## Returns
    ///
    /// A `FileParserCreateResponse` containing the task ID and status.
    pub async fn send_via(&self, client: &ZaiClient) -> ZaiResult<FileParserCreateResponse> {
        debug!(file = %self.file_path.display(), "Creating file parser task");

        let file_bytes = tokio::fs::read(&self.file_path).await?;
        let file_name = self
            .file_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        trace!(bytes = file_bytes.len(), file_name = %file_name, "Prepared parser upload");

        let url = client
            .endpoints()
            .resolve(ApiFamily::PaasV4, &["files", "parser", "create"])?;
        let config = transport_config_from_client(client);
        let tool_type = self.tool_type.clone();
        let file_type = self.file_type.clone();
        let response = send_multipart_request(
            reqwest::Method::POST,
            url,
            client.secret().expose(),
            Arc::new(config),
            move || {
                let file_part = reqwest::multipart::Part::bytes(file_bytes.clone())
                    .file_name(file_name.clone())
                    .mime_str("application/octet-stream")?;
                Ok(reqwest::multipart::Form::new()
                    .part("file", file_part)
                    .text("tool_type", format!("{tool_type:?}").to_lowercase())
                    .text("file_type", format!("{file_type:?}")))
            },
        )
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

fn transport_config_from_client(client: &ZaiClient) -> HttpClientConfig {
    let t = client.transport();
    HttpClientConfig {
        timeout: std::time::Duration::from_secs(t.request_timeout.as_secs()),
        max_retries: u32::from(t.max_attempts).saturating_sub(1),
        enable_compression: t.enable_compression,
        retry_delay: crate::client::http::RetryDelay::Exponential {
            base: std::time::Duration::from_millis(500),
            max: std::time::Duration::from_secs(5),
        },
        enable_logging: false,
        mask_sensitive_data: true,
    }
}
