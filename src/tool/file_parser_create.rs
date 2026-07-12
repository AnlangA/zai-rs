//! Asynchronous file-parser task creation.
//!
//! # Example
//!
//! ```rust,no_run
//! use std::path::Path;
//!
//! use zai_rs::tool::file_parser_create::{FileParseRequest, ToolType};
//! use zai_rs::ZaiClient;
//!
//! # async fn run() -> Result<(), Box<dyn std::error::Error>> {
//!     let client = ZaiClient::from_env()?;
//!     let file_path = Path::new("document.pdf");
//!
//!     let request = FileParseRequest::new(file_path, ToolType::Lite)?;
//!
//!     let response = request.send_via(&client).await?;
//!     if let Some(task_id) = response.task_id() {
//!         println!("Task created: {task_id}");
//!     }
//!
//!     Ok(())
//! }
//! ```

mod data;
mod request;
mod response;

pub use data::FileParseRequest;
pub use request::{FileType, ToolType};
pub use response::FileParserCreateResponse;
