//! File parser creation request models and types.
//!
//! This module provides data structures for file parser creation requests,
//! supporting multiple file formats and parsing tools.

use std::path::Path;

use serde::{Deserialize, Serialize};

/// Parsing tool types with different capabilities.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ToolType {
    /// Lite parser with basic file format support
    Lite,
    /// Expert parser optimized for PDF files
    Expert,
    /// Prime parser with extensive file format support
    Prime,
}

/// Supported file types for parsing.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    /// EPUB files
    EPUB,
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
    /// Check if this file type is supported by the given tool type.
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
            ToolType::Prime => true, // Prime supports all file types
        }
    }

    /// Get the file extension for this file type.
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
            FileType::EPUB => "epub",
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

    /// Try to infer file type from file path.
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
                "epub" => Some(FileType::EPUB),
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
