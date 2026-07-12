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
//! ```text
//! use std::path::Path;
//!
//! use zai_rs::tool::file_parser_create::{FileParserCreateRequest, FileType, ToolType};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let api_key = std::env::var("ZHIPU_API_KEY")?;
//!     let file_path = Path::new("document.pdf");
//!
//!     let request =
//!         FileParserCreateRequest::new(api_key, file_path, ToolType::Lite, FileType::PDF)?;
//!
//!     let response = request.send_via(&client).await?;
//!     println!("Task created: {}", response.task_id);
//!
//!     Ok(())
//! }
//! ```

mod data;
mod request;
mod response;

// Re-export main types for convenience
pub use data::FileParserCreateRequest;
pub use request::{FileType, ToolType};
pub use response::FileParserCreateResponse;
