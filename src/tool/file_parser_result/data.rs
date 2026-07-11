//! # File Parser Result API
//!
//! This module provides the file parser result client for retrieving file
//! parsing results.

use std::sync::Arc;

use tracing::{debug, trace, warn};

use super::{request::*, response::*};
use crate::{
    ZaiResult,
    client::{
        error::codes,
        http::{HttpClientConfig, parse_typed_response, send_empty_request},
        {ApiFamily, ZaiClient},
    },
};

/// File parser result client (P05: routes through [`ZaiClient`]).
///
/// This client provides functionality to retrieve file parsing results,
/// supporting multiple result formats and asynchronous task monitoring.
///
/// ## Examples
///
/// ```rust,ignore
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
        let url = client.endpoints().resolve(
            ApiFamily::PaasV4,
            &[
                "files",
                "parser",
                "result",
                &self.task_id,
                &format_type.to_string(),
            ],
        )?;
        trace!(url = %url, "Fetching file parser result");
        let config = transport_config_from_client(client);
        let response = send_empty_request(
            reqwest::Method::GET,
            url,
            client.secret().expose(),
            Arc::new(config),
        )
        .await?;
        parse_typed_response::<FileParserResultResponse>(response).await
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

fn transport_config_from_client(client: &ZaiClient) -> HttpClientConfig {
    let t = client.transport();
    HttpClientConfig {
        timeout: std::time::Duration::from_secs(t.request_timeout.as_secs()),
        max_retries: u32::from(t.max_attempts).saturating_sub(1),
        enable_compression: t.enable_compression,
        retry_delay: crate::client::http::RetryDelay::Exponential {
            base: std::time::Duration::from_millis(500),
            max: std::time::Duration::from_secs(5),
        },
        enable_logging: false,
        mask_sensitive_data: true,
    }
}
