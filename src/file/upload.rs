use std::{path::PathBuf, sync::Arc};

use super::request::FilePurpose;
use crate::client::{
    http::{HttpClientConfig, parse_typed_response, send_multipart_request},
    {ApiFamily, ZaiClient},
};

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
        let url = client.endpoints().resolve(ApiFamily::PaasV4, &["files"])?;
        let config = transport_config_from_client(client);
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
        let resp = send_multipart_request(
            reqwest::Method::POST,
            url,
            client.secret().expose(),
            Arc::new(config),
            move || {
                let mut part =
                    reqwest::multipart::Part::bytes(bytes.clone()).file_name(fname.clone());
                if let Some(ct) = content_type.as_ref() {
                    part = part.mime_str(ct).map_err(|e| {
                        crate::client::error::ZaiError::ApiError {
                            code: crate::client::error::codes::SDK_VALIDATION,
                            message: format!("invalid content-type: {e}"),
                        }
                    })?;
                }
                Ok(reqwest::multipart::Form::new()
                    .text("purpose", purpose.as_str().to_string())
                    .part("file", part))
            },
        )
        .await?;
        parse_typed_response::<super::response::FileObject>(resp).await
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
