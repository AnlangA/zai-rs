//! File-parser task request builder.

use std::path::Path;

use super::{request::*, response::*};
use crate::{
    ZaiResult,
    client::{ZaiClient, error::codes},
};

/// Request that uploads a local file and creates a parsing task.
///
/// # Examples
///
/// ```rust,no_run
/// use zai_rs::tool::file_parser_create::{FileParseRequest, ToolType};
/// use std::path::Path;
///
/// # fn build() -> zai_rs::ZaiResult<()> {
/// let file_path = Path::new("document.pdf");
///
/// let request = FileParseRequest::new(file_path, ToolType::Lite)?;
/// # let _ = request;
/// # Ok(())
/// # }
/// ```
pub struct FileParseRequest {
    /// Path to the file to parse.
    file_path: std::path::PathBuf,
    /// Parser implementation to use.
    tool_type: ToolType,
    /// Optional declared file type; omission lets the service detect it.
    file_type: Option<FileType>,
}

impl FileParseRequest {
    /// Create a request after verifying that `file_path` is a regular file.
    pub fn new(file_path: &Path, tool_type: ToolType) -> crate::ZaiResult<Self> {
        let metadata = std::fs::symlink_metadata(file_path).map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                crate::client::error::ZaiError::FileError {
                    code: codes::SDK_FILE_NOT_FOUND,
                    message: format!("File does not exist: {}", file_path.display()),
                }
            } else {
                error.into()
            }
        })?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(crate::client::error::ZaiError::FileError {
                code: codes::SDK_FILE_NOT_FOUND,
                message: format!("Path is not a regular file: {}", file_path.display()),
            });
        }

        Ok(Self {
            file_path: file_path.to_path_buf(),
            tool_type,
            file_type: None,
        })
    }

    /// Declare the input file type instead of relying on service detection.
    pub fn with_file_type(mut self, file_type: FileType) -> crate::ZaiResult<Self> {
        if !file_type.is_supported_by(&self.tool_type) {
            return Err(crate::client::validation::invalid(format!(
                "file type {file_type:?} is not supported by tool type {:?}",
                self.tool_type
            )));
        }
        self.file_type = Some(file_type);
        Ok(self)
    }

    /// Create a request and infer the declared file type from the path suffix.
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

        Self::new(file_path, tool_type)?.with_file_type(file_type)
    }

    /// Borrow the local file path.
    pub fn file_path(&self) -> &Path {
        &self.file_path
    }

    /// Return the selected parser implementation.
    pub const fn tool_type(&self) -> ToolType {
        self.tool_type
    }

    /// Return the declared input file type.
    pub const fn file_type(&self) -> Option<FileType> {
        self.file_type
    }

    /// Upload the file and create the parser task through `client`.
    pub async fn send_via(&self, client: &ZaiClient) -> ZaiResult<FileParserCreateResponse> {
        let part =
            crate::client::transport::multipart::FilePart::from_path_async(&self.file_path).await?;

        let route = crate::client::routes::FILES_PARSE;
        let mut factory = crate::client::transport::multipart::MultipartBodyFactory::new()
            .field("tool_type", self.tool_type.as_str())?;
        if let Some(file_type) = self.file_type {
            factory = factory.field("file_type", file_type.extension().to_ascii_uppercase())?;
        }
        let factory = factory.file_named("file", part)?;
        let create_response = client
            .operation(route)
            .send_multipart::<FileParserCreateResponse>(&factory)
            .await
            .map_err(|e| e.context("file parser create"))?;

        if !create_response.is_success() {
            return Err(crate::client::error::ZaiError::ApiError {
                code: codes::SDK_EXTERNAL_TOOL,
                message: create_response.message.as_deref().map_or_else(
                    || "file parser did not return a usable task id".to_owned(),
                    |message| {
                        format!(
                            "file parser task creation failed: {}",
                            crate::client::error::mask_sensitive_info(message)
                        )
                    },
                ),
            });
        }

        Ok(create_response)
    }
}
