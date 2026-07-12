//! # File Management Module
//!
//! Provides file management for the Zhipu AI API: upload, list, content
//! retrieval, and deletion with validation and error handling.
//!
//! # Operations
//!
//! | Operation | Module | Description |
//! |-----------|--------|-------------|
//! | Upload | [`FileUploadRequest`] | Upload a local file |
//! | List | [`FileListRequest`] | List files with metadata |
//! | Content | [`FileContentRequest`] | Retrieve file content |
//! | Delete | [`FileDeleteRequest`] | Delete files |
//! | Parse | [`FileParseSyncRequest`] | Stream a local file to the synchronous parser |
//!
//! # Usage
//!
//! ```rust,no_run
//! use zai_rs::{ZaiResult, client::ZaiClient, file::*};
//!
//! # async fn example(client: &ZaiClient) -> ZaiResult<()> {
//! let uploaded = FileUploadRequest::new(FileUploadPurpose::Agent, "report.pdf")
//!     .send_via(client)
//!     .await?;
//!
//! let files = FileListRequest::new(FileListPurpose::Agent)
//!     .with_query(FileListQuery::new(FileListPurpose::Agent).with_limit(10))
//!     .send_via(client)
//!     .await?;
//!
//! let content = FileContentRequest::new("file-id").send_via(client).await?;
//! let deleted = FileDeleteRequest::new("file-id").send_via(client).await?;
//! let parsed = FileParseSyncRequest::new("report.pdf")
//!     .with_file_type(FileParseSyncFileType::PDF)
//!     .send_via(client)
//!     .await?;
//! # let _ = (uploaded, files, content, deleted, parsed);
//! # Ok(())
//! # }
//! ```

/// Request body / shared types for file operations.
mod request;
/// Response types for file operations.
mod response;

/// Retrieve file content (`GET …/files/{id}/content`).
mod content;
/// Delete a file (`DELETE …/files/{id}`).
mod delete;
/// List files with metadata (`GET …/files`).
mod list;
/// Synchronous file parsing (`POST …/files/parser/sync`).
mod parse_sync;
/// Upload files (`POST …/files`, multipart).
mod upload;

pub use content::*;
pub use delete::*;
pub use list::*;
pub use parse_sync::*;
pub use request::*;
pub use response::*;
pub use upload::*;
