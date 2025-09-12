//! # File Management Module
//!
//! Provides comprehensive file management capabilities for the Zhipu AI API.
//! This module handles file uploads, content retrieval, listing, and deletion
//! operations with proper error handling and validation.
//!
//! ## Module Components
//!
//! - [`content`] - File content retrieval operations
//! - [`delete`] - File deletion functionality
//! - [`list`] - File listing and enumeration
//! - [`request`] - Request types for file operations
//! - [`response`] - Response types for file operations
//! - [`upload`] - File upload functionality
//!
//! ## Supported Operations
//!
//! ### File Upload
//! - Upload files to the Zhipu AI storage system
//! - Support for various file types and formats
//! - Automatic validation and error handling
//!
//! ### File Management
//! - List available files with metadata
//! - Retrieve file content and information
//! - Delete files when no longer needed
//!
//! ## Usage Examples
//!
//!

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
