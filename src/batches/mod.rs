//! Batch processing (batches) module
//!
//! Contains batch-related endpoints (e.g., list, create, retrieve, cancel).
//! Kept flat to reduce nesting while sharing common types.

pub mod cancel;
pub mod create;
pub mod list;
pub mod retrieve;
mod types;

// Re-export selected API types for convenient access via `zai_rs::batches::*`
pub use cancel::{CancelBatchRequest, CancelBatchResponse};
pub use create::{BatchEndpoint, CreateBatchBody, CreateBatchRequest, CreateBatchResponse};
pub use list::{BatchesListQuery, BatchesListRequest, BatchesListResponse, ListObject};
pub use retrieve::{BatchesRetrieveRequest, BatchesRetrieveResponse};
pub use types::BatchItem;
