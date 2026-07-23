//! Synchronous multipart file parsing.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::{ZaiResult, client::ZaiClient};

/// Response returned by synchronous parsing. The sync and result-retrieval
/// endpoints share the frozen `FileParseResultResponse` schema.
pub type FileResponse = crate::tool::FileParseResultResponse;

/// Optional file type accepted by the synchronous `prime-sync` parser.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum FileParseSyncFileType {
    /// WPS document.
    WPS,
    /// PDF document.
    PDF,
    /// Office Open XML Word document.
    DOCX,
    /// Legacy Word document.
    DOC,
    /// Legacy Excel workbook.
    XLS,
    /// Office Open XML Excel workbook.
    XLSX,
    /// Legacy PowerPoint presentation.
    PPT,
    /// Office Open XML PowerPoint presentation.
    PPTX,
    /// PNG image.
    PNG,
    /// JPG image.
    JPG,
    /// JPEG image.
    JPEG,
    /// Comma-separated values file.
    CSV,
    /// Plain-text file.
    TXT,
    /// Markdown file.
    MD,
    /// HTML file.
    HTML,
    /// Bitmap image.
    BMP,
    /// GIF image.
    GIF,
    /// WebP image.
    WEBP,
    /// HEIC image.
    HEIC,
    /// Encapsulated PostScript file.
    EPS,
    /// Apple icon file.
    ICNS,
    /// ImageMagick image.
    IM,
    /// PCX image.
    PCX,
    /// Portable pixmap image.
    PPM,
    /// TIFF image.
    TIFF,
    /// X bitmap image.
    XBM,
    /// HEIF image.
    HEIF,
    /// JPEG 2000 image.
    JP2,
}

impl FileParseSyncFileType {
    /// Return the exact uppercase multipart value.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::WPS => "WPS",
            Self::PDF => "PDF",
            Self::DOCX => "DOCX",
            Self::DOC => "DOC",
            Self::XLS => "XLS",
            Self::XLSX => "XLSX",
            Self::PPT => "PPT",
            Self::PPTX => "PPTX",
            Self::PNG => "PNG",
            Self::JPG => "JPG",
            Self::JPEG => "JPEG",
            Self::CSV => "CSV",
            Self::TXT => "TXT",
            Self::MD => "MD",
            Self::HTML => "HTML",
            Self::BMP => "BMP",
            Self::GIF => "GIF",
            Self::WEBP => "WEBP",
            Self::HEIC => "HEIC",
            Self::EPS => "EPS",
            Self::ICNS => "ICNS",
            Self::IM => "IM",
            Self::PCX => "PCX",
            Self::PPM => "PPM",
            Self::TIFF => "TIFF",
            Self::XBM => "XBM",
            Self::HEIF => "HEIF",
            Self::JP2 => "JP2",
        }
    }
}

/// Synchronous file-parsing request for `POST /files/parser/sync`.
///
/// The parser implementation is fixed to the required wire value
/// `prime-sync`; callers select only the local file and optional file type.
pub struct FileParseSyncRequest {
    file_path: PathBuf,
    file_type: Option<FileParseSyncFileType>,
}

impl std::fmt::Debug for FileParseSyncRequest {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("FileParseSyncRequest")
            .field("file_path", &"[REDACTED]")
            .field("file_type", &self.file_type)
            .finish()
    }
}

impl FileParseSyncRequest {
    /// Create a synchronous parsing request for a local file.
    pub fn new(file_path: impl Into<PathBuf>) -> Self {
        Self {
            file_path: file_path.into(),
            file_type: None,
        }
    }

    /// Set the optional parser file type.
    pub fn with_file_type(mut self, file_type: FileParseSyncFileType) -> Self {
        self.file_type = Some(file_type);
        self
    }

    /// Borrow the local file path.
    pub fn file_path(&self) -> &Path {
        &self.file_path
    }

    /// Return the fixed parser implementation.
    pub const fn tool_type(&self) -> &'static str {
        "prime-sync"
    }

    /// Return the optional declared file type.
    pub const fn file_type(&self) -> Option<FileParseSyncFileType> {
        self.file_type
    }

    /// Validate the local file, stream it as multipart, and decode the parsing
    /// result. The file is reopened and revalidated for the transport attempt.
    pub async fn send_via(&self, client: &ZaiClient) -> ZaiResult<FileResponse> {
        let file_part =
            crate::client::transport::multipart::FilePart::from_path_async(&self.file_path).await?;
        let route = crate::client::routes::FILES_PARSE_SYNC;
        let mut factory = crate::client::transport::multipart::MultipartBodyFactory::new()
            .field("tool_type", self.tool_type())?;
        if let Some(file_type) = self.file_type {
            factory = factory.field("file_type", file_type.as_str())?;
        }
        factory = factory.file_named("file", file_part)?;

        client
            .operation(route)
            .send_multipart::<FileResponse>(&factory)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_type_values_match_the_frozen_enum() {
        let values = [
            FileParseSyncFileType::WPS,
            FileParseSyncFileType::PDF,
            FileParseSyncFileType::DOCX,
            FileParseSyncFileType::DOC,
            FileParseSyncFileType::XLS,
            FileParseSyncFileType::XLSX,
            FileParseSyncFileType::PPT,
            FileParseSyncFileType::PPTX,
            FileParseSyncFileType::PNG,
            FileParseSyncFileType::JPG,
            FileParseSyncFileType::JPEG,
            FileParseSyncFileType::CSV,
            FileParseSyncFileType::TXT,
            FileParseSyncFileType::MD,
            FileParseSyncFileType::HTML,
            FileParseSyncFileType::BMP,
            FileParseSyncFileType::GIF,
            FileParseSyncFileType::WEBP,
            FileParseSyncFileType::HEIC,
            FileParseSyncFileType::EPS,
            FileParseSyncFileType::ICNS,
            FileParseSyncFileType::IM,
            FileParseSyncFileType::PCX,
            FileParseSyncFileType::PPM,
            FileParseSyncFileType::TIFF,
            FileParseSyncFileType::XBM,
            FileParseSyncFileType::HEIF,
            FileParseSyncFileType::JP2,
        ];
        let expected = [
            "WPS", "PDF", "DOCX", "DOC", "XLS", "XLSX", "PPT", "PPTX", "PNG", "JPG", "JPEG", "CSV",
            "TXT", "MD", "HTML", "BMP", "GIF", "WEBP", "HEIC", "EPS", "ICNS", "IM", "PCX", "PPM",
            "TIFF", "XBM", "HEIF", "JP2",
        ];
        assert_eq!(values.map(FileParseSyncFileType::as_str), expected);
        for value in values {
            assert_eq!(
                serde_json::to_value(value).unwrap(),
                serde_json::Value::String(value.as_str().to_owned())
            );
        }
    }

    #[test]
    fn response_required_fields_do_not_default() {
        assert!(
            serde_json::from_value::<FileResponse>(serde_json::json!({
                "status": "succeeded",
                "message": "ok"
            }))
            .is_err()
        );
    }
}
