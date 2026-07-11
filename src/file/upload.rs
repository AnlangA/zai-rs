use std::path::PathBuf;

use super::request::FilePurpose;
use crate::client::ZaiClient;

/// File upload request (multipart/form-data)
///
/// Sends a multipart request with fields:
/// - purpose: `FilePurpose`
/// - file: file content
pub struct FileUploadRequest {
    purpose: FilePurpose,
    file_path: PathBuf,
    file_name: Option<String>,
    content_type: Option<String>,
}

impl FileUploadRequest {
    /// Create a new upload request for the given purpose and local file path.
    pub fn new(purpose: FilePurpose, file_path: impl Into<PathBuf>) -> Self {
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

    /// Send the upload request and parse typed response (`FileObject`)
    pub async fn send_via(
        &self,
        client: &ZaiClient,
    ) -> crate::ZaiResult<super::response::FileObject> {
        let route = crate::client::routes::FILES_UPLOAD;
        let url = client.endpoints().resolve_route(route, &[])?;
        let purpose = self.purpose.clone();
        let path = self.file_path.clone();
        let file_name = self.file_name.clone();
        let content_type = self.content_type.clone();

        let fname = file_name
            .or_else(|| {
                path.file_name()
                    .and_then(|s| s.to_str())
                    .map(std::string::ToString::to_string)
            })
            .unwrap_or_else(|| "upload.bin".to_string());

        let bytes = tokio::fs::read(&path).await?;
        let factory = crate::client::transport::multipart::MultipartBodyFactory::new()
            .field("purpose", purpose.as_str())?
            .bytes_named(
                "file",
                fname,
                content_type.unwrap_or_else(|| "application/octet-stream".to_string()),
                bytes,
            )?;
        client
            .send_multipart::<super::response::FileObject>(route.method(), url, &factory)
            .await
    }
}
