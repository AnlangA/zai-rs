//! # File Parser Result API
//!
//! This module provides the file parser result client for retrieving file parsing results.

use super::{request::*, response::*};
use crate::ZaiResult;
use serde_json;

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
            task_id: task_id.into(),
        }
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
        let url = format!(
            "https://open.bigmodel.cn/api/paas/v4/files/parser/result/{}/{}",
            self.task_id, format_type
        );

        println!("📤 Sending request to: {}", url);
        println!("🔑 Using API key: {}...", &self.key[..10]);

        let client = reqwest::Client::new();
        let response = client.get(&url).bearer_auth(&self.key).send().await?;

        let status = response.status();
        println!("📡 Response status: {}", status);

        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            println!("❌ Error response: {}", error_text);
            return Err(crate::client::error::ZaiError::HttpError {
                status: status.as_u16(),
                message: error_text,
            });
        }

        let response_body = response.text().await?;
        println!("📄 Raw response body: {}", response_body);

        let result_response: FileParserResultResponse = serde_json::from_str(&response_body)?;
        println!("✅ Parsed response: {:?}", result_response);
        Ok(result_response)
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
        println!(
            "⏳ Starting polling for result (timeout: {}s, interval: {}s)",
            timeout_seconds, poll_interval_seconds
        );
        let start_time = std::time::Instant::now();

        loop {
            println!("⏰ Checking result status...");
            let result = self.get_result(format_type.clone()).await?;

            match result.status {
                ParserStatus::Succeeded => {
                    println!("🎉 Parsing completed successfully!");
                    return Ok(result);
                }
                ParserStatus::Failed => {
                    println!("💥 Parsing failed: {}", result.message);
                    return Err(crate::client::error::ZaiError::ApiError {
                        code: 0,
                        message: format!("Parsing failed: {}", result.message),
                    });
                }
                ParserStatus::Processing => {
                    let elapsed = start_time.elapsed().as_secs();
                    println!("⏳ Still processing... ({}s elapsed)", elapsed);
                    if elapsed > timeout_seconds {
                        println!("⏰ Timeout reached!");
                        return Err(crate::client::error::ZaiError::RateLimitError {
                            code: 0,
                            message: "Timeout waiting for parsing result".to_string(),
                        });
                    }
                    println!(
                        "⏱️  Waiting {} seconds before next check...",
                        poll_interval_seconds
                    );
                    tokio::time::sleep(tokio::time::Duration::from_secs(poll_interval_seconds))
                        .await;
                }
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
