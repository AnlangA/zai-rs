//! File parser creation API module for the zai-rs crate.
//!
//! This module provides functionality to create file parsing tasks,
//! supporting multiple file formats and parsing tools.
//!
//! # Example
//!
//! ```no_run
//! use std::path::Path;
//!
//! use zai_rs::tool::file_parser_create::{FileParserCreateRequest, FileType, ToolType};
//! use zai_rs::ZaiClient;
//!
//! # async fn run() -> Result<(), Box<dyn std::error::Error>> {
//!     let client = ZaiClient::from_env()?;
//!     let file_path = Path::new("document.pdf");
//!
//!     let request =
//!         FileParserCreateRequest::new(file_path, ToolType::Lite, FileType::PDF)?;
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

// Keep the task builder and its wire types available from one public module.
pub use data::FileParserCreateRequest;
pub use request::{FileType, ToolType};
pub use response::FileParserCreateResponse;
