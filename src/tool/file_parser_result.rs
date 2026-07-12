//! File parser result API module for the zai-rs crate.
//!
//! This module provides functionality to retrieve file parsing results,
//! supporting multiple result formats and asynchronous task monitoring.
//!
//! # Example
//!
//! ```no_run
//! use zai_rs::tool::file_parser_result::{FileParserResultRequest, FormatType};
//! use zai_rs::ZaiClient;
//!
//! # async fn run() -> Result<(), Box<dyn std::error::Error>> {
//!     let client = ZaiClient::from_env()?;
//!     let task_id = "task_123456789";
//!     let request = FileParserResultRequest::new(task_id);
//!     let response = request.get_result_via(&client, FormatType::Text).await?;
//!     if let Some(content) = response.content() {
//!         println!("Parsed content: {content}");
//!     }
//!
//!     Ok(())
//! }
//! ```

mod data;
mod request;
mod response;

// Keep the result client and its wire types available from one public module.
pub use data::FileParserResultRequest;
pub use request::FormatType;
pub use response::{FileParserResultResponse, ParserStatus};
