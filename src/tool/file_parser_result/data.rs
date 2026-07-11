//! # File Parser Result API
//!
//! This module provides the file parser result client for retrieving file
//! parsing results.

use tracing::{debug, trace, warn};

use super::{request::*, response::*};
use crate::{
    ZaiResult,
    client::{ZaiClient, error::codes},
};

/// File parser result client (P05: routes through [`ZaiClient`]).
///
/// This client provides functionality to retrieve file parsing results,
/// supporting multiple result formats and asynchronous task monitoring.
///
/// ## Examples
///
/// ```text
/// use zai_rs::tool::file_parser_result::{FileParserResultRequest, FormatType};
///
/// let task_id = "task_123456789";
///
/// let request = FileParserResultRequest::new(task_id);
///
/// let response = request.get_result_via(&client, FormatType::Text).await?;
/// if let Some(content) = response.content() {
///     println!("Parsed content: {}", content);
/// }
/// ```
pub struct FileParserResultRequest {
    /// Task ID for the parsing job
    pub task_id: String,
}

impl FileParserResultRequest {
    /// Creates a new file parser result request.
    ///
    /// ## Arguments
    ///
    /// * `task_id` - ID of the parsing task
    ///
    /// ## Returns
    ///
    /// A new `FileParserResultRequest` instance.
    pub fn new(task_id: impl Into<String>) -> Self {
        Self {
            task_id: task_id.into(),
        }
    }

    /// Gets the parsing result for the given format type via a [`ZaiClient`].
    ///
    /// ## Arguments
    ///
    /// * `client` - The [`ZaiClient`] providing credentials and transport
    /// * `format_type` - Format type for the result
    ///
    /// ## Returns
    ///
    /// A `FileParserResultResponse` containing the parsing result.
    pub async fn get_result_via(
        &self,
        client: &ZaiClient,
        format_type: FormatType,
    ) -> ZaiResult<FileParserResultResponse> {
        let route = crate::client::routes::FILES_PARSE_RESULT;
        let format_type = format_type.to_string();
        let url = client
            .endpoints()
            .resolve_route(route, &[&self.task_id, &format_type])?;
        trace!(url = %url, "Fetching file parser result");
        client
            .send_empty::<FileParserResultResponse>(route.method(), url)
            .await
    }

    /// Polls for the result until it's completed or timeout is reached.
    ///
    /// ## Arguments
    ///
    /// * `client` - The [`ZaiClient`] providing credentials and transport
    /// * `format_type` - Format type for the result
    /// * `timeout_seconds` - Maximum time to wait for result
    /// * `poll_interval_seconds` - Interval between status checks
    ///
    /// ## Returns
    ///
    /// A `FileParserResultResponse` containing the parsing result.
    pub async fn wait_for_result_via(
        &self,
        client: &ZaiClient,
        format_type: FormatType,
        timeout_seconds: u64,
        poll_interval_seconds: u64,
    ) -> ZaiResult<FileParserResultResponse> {
        debug!(
            timeout_seconds,
            poll_interval_seconds, "Polling file parser result"
        );
        let start_time = std::time::Instant::now();

        loop {
            trace!("Checking file parser result status");
            let result = self.get_result_via(client, format_type.clone()).await?;

            match result.status {
                ParserStatus::Succeeded => {
                    debug!("File parsing completed successfully");
                    return Ok(result);
                },
                ParserStatus::Failed => {
                    warn!(
                        task_id = %self.task_id,
                        message = %result.message,
                        "File parsing task reported failure"
                    );
                    return Err(crate::client::error::ZaiError::ApiError {
                        code: codes::SDK_EXTERNAL_TOOL,
                        message: format!("Parsing failed: {}", result.message),
                    });
                },
                ParserStatus::Processing => {
                    let elapsed = start_time.elapsed().as_secs();
                    trace!(elapsed, "File parser result still processing");
                    if elapsed > timeout_seconds {
                        warn!(
                            task_id = %self.task_id,
                            elapsed,
                            timeout_seconds,
                            "Polling timed out waiting for parsing result"
                        );
                        return Err(crate::client::error::ZaiError::ApiError {
                            code: codes::SDK_TIMEOUT,
                            message: "Timeout waiting for parsing result".to_string(),
                        });
                    }
                    tokio::time::sleep(tokio::time::Duration::from_secs(poll_interval_seconds))
                        .await;
                },
            }
        }
    }

    /// Gets both text and download link results in a single request.
    ///
    /// ## Arguments
    ///
    /// * `client` - The [`ZaiClient`] providing credentials and transport
    ///
    /// ## Returns
    ///
    /// A tuple containing text result and download link result.
    pub async fn get_all_results_via(
        &self,
        client: &ZaiClient,
    ) -> ZaiResult<(FileParserResultResponse, FileParserResultResponse)> {
        let text_result = self.get_result_via(client, FormatType::Text).await?;
        let download_result = self
            .get_result_via(client, FormatType::DownloadLink)
            .await?;
        Ok((text_result, download_result))
    }
}
