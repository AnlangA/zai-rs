use std::path::{Path, PathBuf};

use validator::Validate;

use super::request::{OcrBody, OcrLanguageType, OcrToolType};
use crate::client::ZaiClient;
use crate::client::error::codes;

const MAX_FILE_SIZE: u64 = 8 * 1024 * 1024;

fn file_error(code: u16, message: impl Into<String>) -> crate::ZaiError {
    crate::ZaiError::FileError {
        code,
        message: message.into(),
    }
}

fn image_mime_type(path: &Path) -> crate::ZaiResult<&'static str> {
    match path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("png") => Ok("image/png"),
        Some("jpg" | "jpeg") => Ok("image/jpeg"),
        Some("bmp") => Ok("image/bmp"),
        extension => Err(file_error(
            codes::SDK_FILE_TYPE_UNSUPPORTED,
            format!(
                "unsupported OCR image extension {extension:?}; expected png, jpg, jpeg, or bmp"
            ),
        )),
    }
}

fn validate_file_metadata(path: &Path, metadata: &std::fs::Metadata) -> crate::ZaiResult<()> {
    if !metadata.is_file() {
        return Err(file_error(
            codes::SDK_FILE_NOT_FOUND,
            format!("file_path is not a regular file: {}", path.display()),
        ));
    }
    if metadata.len() > MAX_FILE_SIZE {
        return Err(file_error(
            codes::SDK_FILE_TOO_LARGE,
            format!(
                "OCR image exceeds the 8 MiB limit: {} bytes",
                metadata.len()
            ),
        ));
    }
    image_mime_type(path).map(|_| ())
}

/// OCR recognition request encoded as `multipart/form-data`.
///
/// Builder for the OCR endpoint. Set the image via
/// [`OcrRequest::with_file_path`], then call [`OcrRequest::send_via`].
pub struct OcrRequest {
    /// Multipart form fields (tool type, language, …).
    pub body: OcrBody,
    file_path: Option<PathBuf>,
}

impl Default for OcrRequest {
    fn default() -> Self {
        Self::new()
    }
}

impl OcrRequest {
    /// Create a new OCR request with default options.
    pub fn new() -> Self {
        Self {
            body: OcrBody::new(),
            file_path: None,
        }
    }

    /// Set the local image file path to recognize (required).
    pub fn with_file_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.file_path = Some(path.into());
        self
    }

    /// Set the OCR tool type (e.g. handwriting).
    pub fn with_tool_type(mut self, tool_type: OcrToolType) -> Self {
        self.body = self.body.with_tool_type(tool_type);
        self
    }

    /// Set the document language.
    pub fn with_language_type(mut self, language_type: OcrLanguageType) -> Self {
        self.body = self.body.with_language_type(language_type);
        self
    }

    /// Whether to return per-character recognition probabilities.
    pub fn with_probability(mut self, probability: bool) -> Self {
        self.body = self.body.with_probability(probability);
        self
    }

    /// Set the client-side request id.
    pub fn with_request_id(mut self, request_id: impl Into<String>) -> Self {
        self.body = self.body.with_request_id(request_id);
        self
    }

    /// Set the end-user id.
    pub fn with_user_id(mut self, user_id: impl Into<String>) -> Self {
        self.body = self.body.with_user_id(user_id);
        self
    }

    /// Validate the request: body constraints, file existence, size (≤8MB),
    /// and supported extension (png/jpg/jpeg/bmp).
    pub fn validate(&self) -> crate::ZaiResult<()> {
        self.body
            .validate()
            .map_err(crate::client::error::ZaiError::from)?;
        let path = self.required_file_path()?;
        let metadata = std::fs::symlink_metadata(path).map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                file_error(
                    codes::SDK_FILE_NOT_FOUND,
                    format!("file_path not found: {}", path.display()),
                )
            } else {
                crate::ZaiError::from(error)
            }
        })?;
        validate_file_metadata(path, &metadata)
    }

    /// Async counterpart of the file probes in [`validate`](Self::validate).
    ///
    /// [`validate`](Self::validate) is a public **sync** fn (kept that way for
    /// backward compatibility) and uses blocking `std::fs` stats; calling it from
    /// `async fn send_via` would stall the executor on a slow/networked filesystem.
    /// This helper performs the same existence/size/extension checks via
    /// `tokio::fs` so the async path stays non-blocking. Keep the two in sync.
    async fn validate_file_async(&self) -> crate::ZaiResult<&Path> {
        let path = self.required_file_path()?;
        let metadata = match tokio::fs::symlink_metadata(path).await {
            Ok(m) => m,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Err(file_error(
                    codes::SDK_FILE_NOT_FOUND,
                    format!("file_path not found: {}", path.display()),
                ));
            },
            Err(e) => return Err(crate::ZaiError::from(e)),
        };
        validate_file_metadata(path, &metadata)?;
        Ok(path)
    }

    fn required_file_path(&self) -> crate::ZaiResult<&Path> {
        self.file_path
            .as_deref()
            .filter(|path| !path.as_os_str().is_empty())
            .ok_or_else(|| crate::client::validation::invalid("file_path is required"))
    }

    /// Submit the multipart request via a [`ZaiClient`] and parse the typed
    /// OCR response.
    pub async fn send_via(
        &self,
        client: &ZaiClient,
    ) -> crate::ZaiResult<super::response::OcrResponse> {
        self.body
            .validate()
            .map_err(crate::client::error::ZaiError::from)?;
        let file_path = self.validate_file_async().await?;

        let route = crate::client::routes::FILES_OCR;
        let url = client.endpoints().resolve_route(route, &[])?;
        let mime = image_mime_type(file_path)?;
        let file_part = crate::client::transport::multipart::FilePart::from_path(file_path)?
            .with_content_type(mime)?;
        let tool_type = self
            .body
            .tool_type
            .unwrap_or(OcrToolType::HandWrite)
            .as_str();
        let language_type = self.body.language_type.map(OcrLanguageType::as_str);
        let probability = self.body.probability;
        let request_id = self.body.request_id.clone();
        let user_id = self.body.user_id.clone();

        let mut factory = crate::client::transport::multipart::MultipartBodyFactory::new()
            .field("tool_type", tool_type)?
            .file_named("file", file_part)?;
        if let Some(lang) = language_type {
            factory = factory.field("language_type", lang)?;
        }
        if let Some(prob) = probability {
            factory = factory.field("probability", prob.to_string())?;
        }
        if let Some(rid) = request_id {
            factory = factory.field("request_id", rid)?;
        }
        if let Some(uid) = user_id {
            factory = factory.field("user_id", uid)?;
        }
        client
            .send_multipart::<super::response::OcrResponse>(route.method(), url, &factory)
            .await
    }
}
