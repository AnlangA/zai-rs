use std::path::PathBuf;

use super::{request::FileUploadPurpose, response::FileUploadResponse};
use crate::client::ZaiClient;

/// File upload request (multipart/form-data)
///
/// Sends a multipart request with fields:
/// - purpose: [`FileUploadPurpose`]
/// - file: file content
pub struct FileUploadRequest {
    purpose: FileUploadPurpose,
    file_path: PathBuf,
    file_name: Option<String>,
    content_type: Option<String>,
}

impl FileUploadRequest {
    /// Create a new upload request for the given purpose and local file path.
    pub fn new(purpose: FileUploadPurpose, file_path: impl Into<PathBuf>) -> Self {
        Self {
            purpose,
            file_path: file_path.into(),
            file_name: None,
            content_type: None,
        }
    }

    /// Override the uploaded file name (defaults to the path's file name).
    pub fn with_file_name(mut self, name: impl Into<String>) -> Self {
        self.file_name = Some(name.into());
        self
    }

    /// Override the MIME content type of the upload.
    pub fn with_content_type(mut self, ct: impl Into<String>) -> Self {
        self.content_type = Some(ct.into());

        self
    }

    /// Send the upload request and parse the returned [`FileObject`](super::response::FileObject).
    ///
    /// The local file is reopened, revalidated, and streamed for each transport
    /// attempt. It is never buffered in full by this request builder.
    pub async fn send_via(&self, client: &ZaiClient) -> crate::ZaiResult<FileUploadResponse> {
        let route = crate::client::routes::FILES_UPLOAD;
        let mut file_part =
            crate::client::transport::multipart::FilePart::from_path_async(&self.file_path).await?;
        if let Some(file_name) = &self.file_name {
            file_part = file_part.with_filename(file_name)?;
        }
        if let Some(content_type) = &self.content_type {
            file_part = file_part.with_content_type(content_type)?;
        }
        let factory = crate::client::transport::multipart::MultipartBodyFactory::new()
            .field("purpose", self.purpose.as_str())?
            .file_named("file", file_part)?;
        client
            .operation(route)
            .send_multipart::<FileUploadResponse>(&factory)
            .await
    }
}
