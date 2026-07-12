//! # Batch Processing Module
//!
//! Provides batch-processing capabilities for the Zhipu AI API. Submit
//! multiple requests asynchronously, inspect job state, and use the returned
//! output/error file identifiers to retrieve results through the file API.
//!
//! # Operations
//!
//! - Create — create a new batch job
//! - List — list batch jobs with filtering
//! - Retrieve — retrieve a batch job's status and results
//! - Cancel — cancel a running or queued batch job
//!
//! # Usage
//!
//! ```rust,no_run
//! use zai_rs::{ZaiResult, batches::*, client::ZaiClient};
//!
//! # async fn example(client: &ZaiClient) -> ZaiResult<()> {
//! let job = BatchCreateRequest::new("batch-input-file-id", BatchEndpoint::ChatCompletions)
//!     .send_via(client)
//!     .await?;
//! let current = BatchGetRequest::new("batch-id").send_via(client).await?;
//! let cancelled = BatchCancelRequest::new("batch-id").send_via(client).await?;
//! # let _ = (job, current, cancelled);
//! # Ok(())
//! # }
//! ```

/// Cancel a running or queued batch job (`POST …/batches/{id}/cancel`).
mod cancel;
/// Create a new batch job (`POST …/batches`).
mod create;
/// List batch jobs with filtering/pagination (`GET …/batches`).
mod list;
/// Retrieve a batch job's status and results (`GET …/batches/{id}`).
mod retrieve;
mod types;

pub use cancel::{BatchCancelRequest, BatchCancelResponse};
pub use create::{BatchCreateBody, BatchCreateRequest, BatchCreateResponse, BatchEndpoint};
pub use list::{BatchListObject, BatchListQuery, BatchListRequest, BatchListResponse};
pub use retrieve::{BatchGetRequest, BatchGetResponse};
pub use types::BatchItem;
