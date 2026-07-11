use std::path::Path;

use validator::Validate;

use super::request::{OcrBody, OcrLanguageType, OcrToolType};
use crate::client::ZaiClient;
use crate::client::error::codes;

/// OCR recognition request (multipart/form-data)
///
/// Builder for the OCR endpoint. Set the image via
/// [`OcrRequest::with_file_path`], then call [`OcrRequest::send_via`].
///
/// (plan P05: migrated to route through [`ZaiClient`].)
pub struct OcrRequest {
    /// Multipart form fields (tool type, language, …).
    pub body: OcrBody,
    file_path: Option<String>,
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
    pub fn with_file_path(mut self, path: impl Into<String>) -> Self {
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
        // Check body constraints
        self.body
            .validate()
            .map_err(crate::client::error::ZaiError::from)?;

        // Ensure file path exists
        let p =
            self.file_path
                .as_ref()
                .ok_or_else(|| crate::client::error::ZaiError::ApiError {
                    code: crate::client::error::codes::SDK_VALIDATION,
                    message: "file_path is required".to_string(),
                })?;

        if !Path::new(p).exists() {
            return Err(crate::client::error::ZaiError::FileError {
                code: codes::SDK_FILE_NOT_FOUND,
                message: format!("file_path not found: {p}"),
            });
        }

        // Validate file size (max 8MB)
        let metadata = std::fs::metadata(p)?;
        let file_size = metadata.len();
        const MAX_SIZE: u64 = 8 * 1024 * 1024; // 8MB
        if file_size > MAX_SIZE {
            return Err(crate::client::error::ZaiError::FileError {
                code: codes::SDK_FILE_TOO_LARGE,
                message: format!("file_size exceeds 8MB limit: {file_size} bytes"),
            });
        }

        // Validate file extension
        let ext = Path::new(p)
            .extension()
            .and_then(|s| s.to_str())
            .map(str::to_ascii_lowercase);
        let valid_ext = matches!(ext.as_deref(), Some("png" | "jpg" | "jpeg" | "bmp"));
        if !valid_ext {
            return Err(crate::client::error::ZaiError::FileError {
                code: codes::SDK_FILE_TYPE_UNSUPPORTED,
                message: format!(
                    "invalid file format: {ext:?}. Only PNG, JPG, JPEG, BMP are supported"
                ),
            });
        }

        Ok(())
    }

    /// Async counterpart of the file probes in [`validate`](Self::validate).
    ///
    /// [`validate`](Self::validate) is a public **sync** fn (kept that way for
    /// backward compatibility) and uses blocking `std::fs` stats; calling it from
    /// `async fn send_via` would stall the executor on a slow/networked filesystem.
    /// This helper performs the same existence/size/extension checks via
    /// `tokio::fs` so the async path stays non-blocking. Keep the two in sync.
    async fn validate_file_async(&self) -> crate::ZaiResult<()> {
        let p =
            self.file_path
                .as_ref()
                .ok_or_else(|| crate::client::error::ZaiError::ApiError {
                    code: crate::client::error::codes::SDK_VALIDATION,
                    message: "file_path is required".to_string(),
                })?;

        // A single async `metadata` call covers both existence (a missing path
        // yields `NotFound`) and size, replacing the sync `Path::exists()` +
        // `std::fs::metadata` pair.
        let metadata = match tokio::fs::metadata(p).await {
            Ok(m) => m,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Err(crate::client::error::ZaiError::FileError {
                    code: codes::SDK_FILE_NOT_FOUND,
                    message: format!("file_path not found: {p}"),
                });
            },
            Err(e) => return Err(crate::client::error::ZaiError::from(e)),
        };

        const MAX_SIZE: u64 = 8 * 1024 * 1024; // 8MB
        let file_size = metadata.len();
        if file_size > MAX_SIZE {
            return Err(crate::client::error::ZaiError::FileError {
                code: codes::SDK_FILE_TOO_LARGE,
                message: format!("file_size exceeds 8MB limit: {file_size} bytes"),
            });
        }

        // Pure-path extension check (no I/O).
        let ext = Path::new(p)
            .extension()
            .and_then(|s| s.to_str())
            .map(str::to_ascii_lowercase);
        let valid_ext = matches!(ext.as_deref(), Some("png" | "jpg" | "jpeg" | "bmp"));
        if !valid_ext {
            return Err(crate::client::error::ZaiError::FileError {
                code: codes::SDK_FILE_TYPE_UNSUPPORTED,
                message: format!(
                    "invalid file format: {ext:?}. Only PNG, JPG, JPEG, BMP are supported"
                ),
            });
        }
        Ok(())
    }

    /// Submit the multipart request via a [`ZaiClient`] and parse the typed
    /// OCR response.
    pub async fn send_via(
        &self,
        client: &ZaiClient,
    ) -> crate::ZaiResult<super::response::OcrResponse> {
        // Field-level validation is cheap and I/O-free; run it inline. The file
        // probes go through `validate_file_async` so no blocking std::fs stat
        // runs on the async executor (the public sync `validate()` still does).
        self.body
            .validate()
            .map_err(crate::client::error::ZaiError::from)?;
        self.validate_file_async().await?;

        let url = client
            .endpoints()
            .resolve(crate::client::ApiFamily::PaasV4, &["files", "ocr"])?;
        let file_path =
            self.file_path
                .clone()
                .ok_or_else(|| crate::client::error::ZaiError::ApiError {
                    code: crate::client::error::codes::SDK_VALIDATION,
                    message: "file_path is required".to_string(),
                })?;

        let file_name = Path::new(&file_path)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("image.png")
            .to_string();
        let file_bytes = tokio::fs::read(&file_path).await?;

        // Determine MIME type by extension
        let ext = Path::new(&file_path)
            .extension()
            .and_then(|s| s.to_str())
            .map(str::to_ascii_lowercase);
        let mime = match ext.as_deref() {
            Some("png") => "image/png",
            Some("jpg" | "jpeg") => "image/jpeg",
            Some("bmp") => "image/bmp",
            _ => "image/png",
        };

        let tool_type_str = match &self.body.tool_type {
            Some(OcrToolType::HandWrite) => "hand_write",
            None => "hand_write",
        }
        .to_string();

        let language_type = self.body.language_type.as_ref().map(|lang| {
            serde_json::to_string(lang)
                .unwrap_or_default()
                .trim_matches('"')
                .to_string()
        });
        let probability = self.body.probability;
        let request_id = self.body.request_id.clone();
        let user_id = self.body.user_id.clone();

        let mut factory = crate::client::transport::multipart::MultipartBodyFactory::new()
            .field("tool_type", tool_type_str)?
            .bytes_named("file", file_name, mime, file_bytes)?;
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
            .send_multipart::<super::response::OcrResponse>("POST", url, &factory)
            .await
    }
}
