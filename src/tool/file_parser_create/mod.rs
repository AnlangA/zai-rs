//! File parser creation API module for the zai-rs crate.
//!
//! This module provides functionality to create file parsing tasks,
//! supporting multiple file formats and parsing tools.
//!
//! # Features
//!
//! - Multiple parsing tools (lite, expert, prime)
//! - Support for various file formats (PDF, DOCX, XLSX, images, etc.)
//! - Comprehensive validation
//! - Type-safe request and response models
//!
//! # Example
//!
//! ```rust
//! use zai_rs::tool::file_parser_create::{FileParserCreateRequest, ToolType, FileType};
//! use std::path::Path;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let api_key = std::env::var("ZHIPU_API_KEY")?;
//!     let file_path = Path::new("document.pdf");
//!
//!     let request = FileParserCreateRequest::new(
//!         api_key,
//!         file_path,
//!         ToolType::Lite,
//!         FileType::PDF,
//!     )?;
//!
//!     let response = request.send().await?;
//!     println!("Task created: {}", response.task_id);
//!
//!     Ok(())
//! }
//! ```

pub mod data;
pub mod request;
pub mod response;

// Re-export main types for convenience
pub use data::FileParserCreateRequest;
pub use request::{FileType, ToolType};
pub use response::FileParserCreateResponse;
