//! File parser result API module for the zai-rs crate.
//!
//! This module provides functionality to retrieve file parsing results,
//! supporting multiple result formats and asynchronous task monitoring.
//!
//! # Features
//!
//! - Multiple result formats (text, download_link)
//! - Task status monitoring
//! - Asynchronous result retrieval
//! - Polling support with timeout
//! - Comprehensive error handling
//!
//! # Example
//!
//! ```rust
//! use zai_rs::tool::file_parser_result::{FileParserResultRequest, FormatType};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let api_key = std::env::var("ZHIPU_API_KEY")?;
//!     let task_id = "task_123456789";
//!
//!     let request = FileParserResultRequest::new(api_key, task_id);
//!
//!     let response = request.get_result(FormatType::Text).await?;
//!     if let Some(content) = response.content() {
//!         println!("Parsed content: {}", content);
//!     }
//!
//!     Ok(())
//! }
//! ```

pub mod data;
pub mod request;
pub mod response;

// Re-export main types for convenience
pub use data::FileParserResultRequest;
pub use request::FormatType;
pub use response::{FileParserResultResponse, ParserStatus};
