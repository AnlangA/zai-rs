//! # File Management Module
//!
//! Provides file management for the Zhipu AI API: upload, list, content
//! retrieval, and deletion with validation and error handling.
//!
//! # Operations
//!
//! | Operation | Module | Description |
//! |-----------|--------|-------------|
//! | Upload | [`upload`] | Upload files (PDF, images, etc.) |
//! | List | [`list`] | List files with metadata |
//! | Content | [`content`] | Retrieve file content |
//! | Delete | [`delete`] | Delete files |
//!
//! # Usage
//!
//! ```rust,ignore
//! use zai_rs::file::*;
//!
//! // Upload
//! let result = client.upload_file(&FileUploadRequest::new(file, ContentType::Pdf)).await?;
//!
//! // List
//! let files = client.list_files(&FileListRequest::new().limit(10)).await?;
//!
//! // Get content
//! let content = client.get_file_content(&FileContentRequest::new(file_id)).await?;
//!
//! // Delete
//! client.delete_file(&FileDeleteRequest::new(file_id)).await?;
//! ```

/// Request body / shared types for file operations.
pub mod request;
/// Response types for file operations.
pub mod response;

// Split operations into clear modules
/// Retrieve file content (`GET …/files/{id}/content`).
pub mod content;
/// Delete a file (`DELETE …/files/{id}`).
pub mod delete;
/// List files with metadata (`GET …/files`).
pub mod list;
/// Synchronous file parsing (`POST …/files/parser/sync`, P06).
pub mod parse_sync;
/// Upload files (`POST …/files`, multipart).
pub mod upload;

pub use content::*;
pub use delete::*;
pub use list::*;
pub use parse_sync::*;
pub use request::*;
pub use response::*;
pub use upload::*;
