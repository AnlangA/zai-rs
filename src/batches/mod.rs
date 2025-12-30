//! # Batch Processing Module
//!
//! Provides batch processing capabilities for the Zhipu AI API.
//! Batch processing allows you to submit multiple requests asynchronously
//! and retrieve results at a later time, useful for high-volume operations.
//!
//! ## Module Components
//!
//! - [`cancel`] - Cancel running batch operations
//! - [`create`] - Create new batch processing jobs
//! - [`list`] - List batch operations with filtering
//! - [`retrieve`] - Retrieve batch operation results
//! - [`types`] - Shared data types for batch operations
//!
//! ## Supported Operations
//!
//! ### Create Batch Job
//! Submit multiple requests for asynchronous processing:
//! ```rust,ignore
//! use zai_rs::batches::{CreateBatchRequest, CreateBatchBody};
//! use zai_rs::model::{ChatCompletionRequest, GLM4_5_flash};
//!
//! let body = CreateBatchBody {
//!     endpoint: BatchEndpoint::ChatCompletions,
//!     completion_window: "24h".to_string(),
//!     requests: vec![/* ... requests ... */],
//! };
//!
//! let request = CreateBatchRequest::new(body);
//! let response = client.create_batch(&request).await?;
//! ```
//!
//! ### List Batch Jobs
//! Query batch operations with filters:
//! ```rust,ignore
//! use zai_rs::batches::{BatchesListRequest, BatchesListQuery};
//!
//! let query = BatchesListQuery {
//!     limit: Some(20),
//!     after: Some("batch_abc123".to_string()),
//! };
//!
//! let request = BatchesListRequest::new(query);
//! let response = client.list_batches(&request).await?;
//! ```
//!
//! ### Retrieve Batch Result
//! Get the results of a completed batch:
//! ```rust,ignore
//! use zai_rs::batches::BatchesRetrieveRequest;
//!
//! let request = BatchesRetrieveRequest::new("batch_abc123");
//! let response = client.retrieve_batch(&request).await?;
//! ```
//!
//! ### Cancel Batch Job
//! Cancel a running or queued batch:
//! ```rust,ignore
//! use zai_rs::batches::CancelBatchRequest;
//!
//! let request = CancelBatchRequest::new("batch_abc123");
//! let response = client.cancel_batch(&request).await?;
//! ```
//!
//! ## Use Cases
//!
//! - **High-Volume Processing**: Process thousands of requests efficiently
//! - **Scheduled Processing**: Submit requests now, process during off-peak
//!   hours
//! - **Cost Optimization**: Take advantage of batch pricing tiers
//!
//! ## Batch Lifecycle
//!
//! 1. **Initializing**: Batch is being validated
//! 2. **In Progress**: Batch is being processed
//! 3. **Completed**: All requests processed successfully
//! 4. **Failed**: Batch encountered errors during processing
//! 5. **Cancelled**: Batch was cancelled by the user
//! 6. **Expired**: Batch results expired (after 24 hours by default)

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
