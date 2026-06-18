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
        endpoints::{ApiBase, EndpointConfig, join_url, paths},
        error::codes,
        http::{HttpClientConfig, parse_typed_response, send_empty_request},
    },
};

/// File parser result client.
///
/// This client provides functionality to retrieve file parsing results,
/// supporting multiple result formats and asynchronous task monitoring.
///
/// ## Examples
///
/// ```rust,ignore
/// use zai_rs::tool::file_parser_result::{FileParserResultRequest, FormatType};
///
/// let api_key = "your-api-key".to_string();
/// let task_id = "task_123456789";
///
/// let request = FileParserResultRequest::new(api_key, task_id);
///
/// let response = request.get_result(FormatType::Text).await?;
/// if let Some(content) = response.content() {
///     println!("Parsed content: {}", content);
/// }
/// ```
pub struct FileParserResultRequest {
    /// API key for authentication
    pub key: String,
    endpoint_config: EndpointConfig,
    api_base: ApiBase,
    http_config: Arc<HttpClientConfig>,
    /// Task ID for the parsing job
    pub task_id: String,
}

impl FileParserResultRequest {
    /// Creates a new file parser result request.
    ///
    /// ## Arguments
    ///
    /// * `key` - API key for authentication
    /// * `task_id` - ID of the parsing task
    ///
    /// ## Returns
    ///
    /// A new `FileParserResultRequest` instance.
    pub fn new(key: String, task_id: impl Into<String>) -> Self {
        Self {
            key,
            endpoint_config: EndpointConfig::default(),
            api_base: ApiBase::PaasV4,
            http_config: Arc::new(HttpClientConfig::default()),
            task_id: task_id.into(),
        }
    }

    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.api_base = ApiBase::Custom(base_url.into());
        self
    }

    pub fn with_endpoint_config(mut self, endpoint_config: EndpointConfig) -> Self {
        self.endpoint_config = endpoint_config;
        self
    }

    pub fn with_http_config(mut self, config: HttpClientConfig) -> Self {
        self.http_config = Arc::new(config);
        self
    }

    fn result_url(&self, format_type: &FormatType) -> String {
        let path = join_url(
            paths::FILE_PARSER_RESULT,
            &join_url(&self.task_id, &format_type.to_string()),
        );
        self.endpoint_config.url(&self.api_base, &path)
    }

    /// Gets the parsing result for the given format type.
    ///
    /// ## Arguments
    ///
    /// * `format_type` - Format type for the result
    ///
    /// ## Returns
    ///
    /// A `FileParserResultResponse` containing the parsing result.
    pub async fn get_result(&self, format_type: FormatType) -> ZaiResult<FileParserResultResponse> {
        let url = self.result_url(&format_type);
        trace!(url = %url, "Fetching file parser result");
        let response = send_empty_request(
            reqwest::Method::GET,
            url,
            self.key.clone(),
            self.http_config.clone(),
        )
        .await?;
        parse_typed_response::<FileParserResultResponse>(response).await
    }

    /// Polls for the result until it's completed or timeout is reached.
    ///
    /// ## Arguments
    ///
    /// * `format_type` - Format type for the result
    /// * `timeout_seconds` - Maximum time to wait for result
    /// * `poll_interval_seconds` - Interval between status checks
    ///
    /// ## Returns
    ///
    /// A `FileParserResultResponse` containing the parsing result.
    pub async fn wait_for_result(
        &self,
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
            let result = self.get_result(format_type.clone()).await?;

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
    /// ## Returns
    ///
    /// A tuple containing text result and download link result.
    pub async fn get_all_results(
        &self,
    ) -> ZaiResult<(FileParserResultResponse, FileParserResultResponse)> {
        let text_result = self.get_result(FormatType::Text).await?;
        let download_result = self.get_result(FormatType::DownloadLink).await?;
        Ok((text_result, download_result))
    }
}
