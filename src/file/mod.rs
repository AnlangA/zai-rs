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

pub mod request;
pub mod response;

// Split operations into clear modules
pub mod content;
pub mod delete;
pub mod list;
pub mod upload;

pub use content::*;
pub use delete::*;
pub use list::*;
pub use request::*;
pub use response::*;
pub use upload::*;
