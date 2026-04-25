//! # Batch Processing Module
//!
//! Provides batch-processing capabilities for the Zhipu AI API. Submit
//! multiple requests asynchronously and retrieve results later — ideal for
//! high-volume or scheduled workloads.
//!
//! # Operations
//!
//! - [`create`] — Create a new batch job
//! - [`list`] — List batch jobs with filtering
//! - [`retrieve`] — Retrieve a batch job's status and results
//! - [`cancel`] — Cancel a running or queued batch job
//!
//! # Batch Lifecycle
//!
//! 1. `Initializing` — Job is being validated
//! 2. `In Progress` — Job is being processed
//! 3. `Completed` — All requests processed successfully
//! 4. `Failed` — Job encountered errors
//! 5. `Cancelled` — Job was cancelled by the user
//! 6. `Expired` — Results expired (default: 24 h)
//!
//! # Usage
//!
//! ```rust,ignore
//! use zai_rs::batches::*;
//!
//! // Create
//! let body = CreateBatchBody { endpoint: BatchEndpoint::ChatCompletions, .. };
//! let job = client.create_batch(&CreateBatchRequest::new(body)).await?;
//!
//! // Retrieve
//! let result = client.retrieve_batch(&BatchesRetrieveRequest::new(&job.id)).await?;
//!
//! // Cancel
//! client.cancel_batch(&CancelBatchRequest::new(&job.id)).await?;
//! ```

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
