//! File-parser request wire types.

use std::path::Path;

use serde::{Deserialize, Serialize};

/// Parser implementation selected for a task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ToolType {
    /// Lite parser with basic file-format support.
    Lite,
    /// Expert parser optimized for PDF files.
    Expert,
    /// Prime parser with the complete supported format set.
    Prime,
}

impl ToolType {
    /// Canonical value sent to the parser API.
    pub fn as_str(self) -> &'static str {
        match self {
            ToolType::Lite => "lite",
            ToolType::Expert => "expert",
            ToolType::Prime => "prime",
        }
    }
}

/// Supported file types for parsing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum FileType {
    /// PDF documents
    PDF,
    /// Word documents (.docx)
    DOCX,
    /// Word documents (.doc)
    DOC,
    /// Excel spreadsheets (.xls)
    XLS,
    /// Excel spreadsheets (.xlsx)
    XLSX,
    /// PowerPoint presentations (.ppt)
    PPT,
    /// PowerPoint presentations (.pptx)
    PPTX,
    /// PNG images
    PNG,
    /// JPG images
    JPG,
    /// JPEG images
    JPEG,
    /// CSV files
    CSV,
    /// Text files
    TXT,
    /// Markdown files
    MD,
    /// HTML files
    HTML,
    /// BMP images
    BMP,
    /// GIF images
    GIF,
    /// WEBP images
    WEBP,
    /// HEIC images
    HEIC,
    /// EPS files
    EPS,
    /// ICNS files
    ICNS,
    /// IM images
    IM,
    /// PCX images
    PCX,
    /// PPM images
    PPM,
    /// TIFF images
    TIFF,
    /// XBM images
    XBM,
    /// HEIF images
    HEIF,
    /// JP2 images
    JP2,
}

impl FileType {
    /// Return whether `tool_type` accepts this file type.
    pub fn is_supported_by(&self, tool_type: &ToolType) -> bool {
        match tool_type {
            ToolType::Lite => {
                matches!(
                    self,
                    FileType::PDF
                        | FileType::DOCX
                        | FileType::DOC
                        | FileType::XLS
                        | FileType::XLSX
                        | FileType::PPT
                        | FileType::PPTX
                        | FileType::PNG
                        | FileType::JPG
                        | FileType::JPEG
                        | FileType::CSV
                        | FileType::TXT
                        | FileType::MD
                )
            },
            ToolType::Expert => matches!(self, FileType::PDF),
            ToolType::Prime => true,
        }
    }

    /// Return the canonical lowercase file extension.
    pub fn extension(&self) -> &'static str {
        match self {
            FileType::PDF => "pdf",
            FileType::DOCX => "docx",
            FileType::DOC => "doc",
            FileType::XLS => "xls",
            FileType::XLSX => "xlsx",
            FileType::PPT => "ppt",
            FileType::PPTX => "pptx",
            FileType::PNG => "png",
            FileType::JPG => "jpg",
            FileType::JPEG => "jpeg",
            FileType::CSV => "csv",
            FileType::TXT => "txt",
            FileType::MD => "md",
            FileType::HTML => "html",
            FileType::BMP => "bmp",
            FileType::GIF => "gif",
            FileType::WEBP => "webp",
            FileType::HEIC => "heic",
            FileType::EPS => "eps",
            FileType::ICNS => "icns",
            FileType::IM => "im",
            FileType::PCX => "pcx",
            FileType::PPM => "ppm",
            FileType::TIFF => "tiff",
            FileType::XBM => "xbm",
            FileType::HEIF => "heif",
            FileType::JP2 => "jp2",
        }
    }

    /// Infer a file type from the final path extension.
    pub fn from_path(path: &Path) -> Option<Self> {
        path.extension()
            .and_then(|ext| ext.to_str())
            .and_then(|ext_str| match ext_str.to_lowercase().as_str() {
                "pdf" => Some(FileType::PDF),
                "docx" => Some(FileType::DOCX),
                "doc" => Some(FileType::DOC),
                "xls" => Some(FileType::XLS),
                "xlsx" => Some(FileType::XLSX),
                "ppt" => Some(FileType::PPT),
                "pptx" => Some(FileType::PPTX),
                "png" => Some(FileType::PNG),
                "jpg" => Some(FileType::JPG),
                "jpeg" => Some(FileType::JPEG),
                "csv" => Some(FileType::CSV),
                "txt" => Some(FileType::TXT),
                "md" => Some(FileType::MD),
                "html" => Some(FileType::HTML),
                "htm" => Some(FileType::HTML),
                "bmp" => Some(FileType::BMP),
                "gif" => Some(FileType::GIF),
                "webp" => Some(FileType::WEBP),
                "heic" => Some(FileType::HEIC),
                "eps" => Some(FileType::EPS),
                "icns" => Some(FileType::ICNS),
                "im" => Some(FileType::IM),
                "pcx" => Some(FileType::PCX),
                "ppm" => Some(FileType::PPM),
                "tiff" => Some(FileType::TIFF),
                "tif" => Some(FileType::TIFF),
                "xbm" => Some(FileType::XBM),
                "heif" => Some(FileType::HEIF),
                "jp2" => Some(FileType::JP2),
                _ => None,
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parser_file_types_match_the_frozen_enum_exactly() {
        let values = [
            FileType::PDF,
            FileType::DOCX,
            FileType::DOC,
            FileType::XLS,
            FileType::XLSX,
            FileType::PPT,
            FileType::PPTX,
            FileType::PNG,
            FileType::JPG,
            FileType::JPEG,
            FileType::CSV,
            FileType::TXT,
            FileType::MD,
            FileType::HTML,
            FileType::BMP,
            FileType::GIF,
            FileType::WEBP,
            FileType::HEIC,
            FileType::EPS,
            FileType::ICNS,
            FileType::IM,
            FileType::PCX,
            FileType::PPM,
            FileType::TIFF,
            FileType::XBM,
            FileType::HEIF,
            FileType::JP2,
        ];
        let actual = values.map(|value| serde_json::to_value(value).unwrap());
        let expected = [
            "PDF", "DOCX", "DOC", "XLS", "XLSX", "PPT", "PPTX", "PNG", "JPG", "JPEG", "CSV", "TXT",
            "MD", "HTML", "BMP", "GIF", "WEBP", "HEIC", "EPS", "ICNS", "IM", "PCX", "PPM", "TIFF",
            "XBM", "HEIF", "JP2",
        ]
        .map(|value| serde_json::Value::String(value.to_owned()));
        assert_eq!(actual, expected);
    }

    #[test]
    fn parser_tool_capabilities_reject_unsupported_combinations() {
        assert!(FileType::TXT.is_supported_by(&ToolType::Lite));
        assert!(!FileType::HTML.is_supported_by(&ToolType::Lite));
        assert!(FileType::PDF.is_supported_by(&ToolType::Expert));
        assert!(!FileType::DOCX.is_supported_by(&ToolType::Expert));
        assert!(FileType::JP2.is_supported_by(&ToolType::Prime));
    }
}
