//! # File Parser Creation API
//!
//! This module provides the file parser creation client for creating file parsing tasks.

use super::{request::*, response::*};
use serde_json;
use std::path::Path;

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
    /// A new `FileParserCreateRequest` instance or an error if validation fails.
    pub fn new(
        key: String,
        file_path: &Path,
        tool_type: ToolType,
        file_type: FileType,
    ) -> anyhow::Result<Self> {
        // Validate that file exists
        if !file_path.exists() {
            return Err(anyhow::anyhow!(
                "File does not exist: {}",
                file_path.display()
            ));
        }

        // Validate that file type is supported by tool
        if !file_type.is_supported_by(&tool_type) {
            return Err(anyhow::anyhow!(
                "File type {:?} is not supported by tool type {:?}",
                file_type,
                tool_type
            ));
        }

        Ok(Self {
            key,
            file_path: file_path.to_path_buf(),
            tool_type,
            file_type,
        })
    }

    /// Creates a new file parser creation request with automatic file type detection.
    ///
    /// ## Arguments
    ///
    /// * `key` - API key for authentication
    /// * `file_path` - Path to the file to parse
    /// * `tool_type` - Type of parsing tool to use
    ///
    /// ## Returns
    ///
    /// A new `FileParserCreateRequest` instance or an error if validation fails.
    pub fn new_with_auto_type(
        key: String,
        file_path: &Path,
        tool_type: ToolType,
    ) -> anyhow::Result<Self> {
        let file_type = FileType::from_path(file_path).ok_or_else(|| {
            anyhow::anyhow!(
                "Could not determine file type from path: {}",
                file_path.display()
            )
        })?;

        Self::new(key, file_path, tool_type, file_type)
    }

    /// Sends the file parser task creation request.
    ///
    /// ## Returns
    ///
    /// A `FileParserCreateResponse` containing the task ID and status.
    pub async fn send(&self) -> anyhow::Result<FileParserCreateResponse> {
        println!("📤 Creating file parser task...");
        println!("📁 File: {}", self.file_path.display());
        println!("🛠️  Tool type: {:?}", self.tool_type);
        println!("📄 File type: {:?}", self.file_type);
        println!("🔑 API key: {}...", &self.key[..10]);

        let file_bytes = tokio::fs::read(&self.file_path).await?;
        let file_name = self
            .file_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        println!("📊 File size: {} bytes", file_bytes.len());
        println!("📝 File name: {}", file_name);

        let file_part = reqwest::multipart::Part::bytes(file_bytes)
            .file_name(file_name)
            .mime_str("application/octet-stream")?;

        let form = reqwest::multipart::Form::new()
            .part("file", file_part)
            .text("tool_type", format!("{:?}", self.tool_type).to_lowercase())
            .text("file_type", format!("{:?}", self.file_type));

        let client = reqwest::Client::new();
        println!("🌐 Sending request to: https://open.bigmodel.cn/api/paas/v4/files/parser/create");

        let response = client
            .post("https://open.bigmodel.cn/api/paas/v4/files/parser/create")
            .bearer_auth(&self.key)
            .multipart(form)
            .send()
            .await?;

        let status = response.status();
        println!("📡 Response status: {}", status);

        let response_text = response.text().await.unwrap_or_default();
        println!("📄 Raw response: {}", response_text);

        if !status.is_success() {
            return Err(anyhow::anyhow!("HTTP {} - {}", status, response_text));
        }

        let create_response: FileParserCreateResponse = serde_json::from_str(&response_text)
            .map_err(|e| {
                anyhow::anyhow!(
                    "Failed to decode response: {} - Response was: {}",
                    e,
                    response_text
                )
            })?;

        println!("✅ Task created successfully: {:?}", create_response);

        if !create_response.is_success() {
            return Err(anyhow::anyhow!(
                "Task creation failed: {}",
                create_response.message
            ));
        }

        Ok(create_response)
    }
}
