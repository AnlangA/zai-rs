//! File-parser result retrieval and polling.
//!
//! # Example
//!
//! ```rust,no_run
//! use zai_rs::tool::file_parser_result::{FileParseResultRequest, FormatType};
//! use zai_rs::ZaiClient;
//!
//! # async fn run() -> Result<(), Box<dyn std::error::Error>> {
//!     let client = ZaiClient::from_env()?;
//!     let task_id = "task_123456789";
//!     let request = FileParseResultRequest::new(task_id);
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

pub use data::FileParseResultRequest;
pub use request::FormatType;
pub use response::{FileParseResultResponse, ParserStatus};
