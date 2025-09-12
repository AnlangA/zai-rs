//! Batch processing (batches) module
//!
//! Contains batch-related endpoints (e.g., list, create, retrieve, cancel).
//! Kept flat to reduce nesting while sharing common types.

mod types;
pub mod list;
pub mod create;
pub mod retrieve;
pub mod cancel;

// Re-export selected API types for convenient access via `zai_rs::batches::*`
pub use types::BatchItem;
pub use list::{BatchesListQuery, BatchesListRequest, BatchesListResponse, ListObject};
pub use create::{BatchEndpoint, CreateBatchBody, CreateBatchRequest, CreateBatchResponse};
pub use retrieve::{BatchesRetrieveRequest, BatchesRetrieveResponse};
pub use cancel::{CancelBatchRequest, CancelBatchResponse};

