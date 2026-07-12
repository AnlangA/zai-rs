//! File-parser result request and polling helpers.

use tracing::{debug, trace, warn};

use super::{request::*, response::*};
use crate::{
    ZaiResult,
    client::{ZaiClient, error::codes},
};

/// Handle for retrieving or polling one file-parsing task.
///
/// # Examples
///
/// ```rust,no_run
/// use zai_rs::tool::file_parser_result::{FileParseResultRequest, FormatType};
/// use zai_rs::ZaiClient;
///
/// # async fn fetch(client: &ZaiClient) -> zai_rs::ZaiResult<()> {
/// let request = FileParseResultRequest::new("task_123456789");
/// let response = request.get_result_via(client, FormatType::Text).await?;
/// if let Some(content) = response.content() {
///     println!("Parsed content: {}", content);
/// }
/// # Ok(())
/// # }
/// ```
pub struct FileParseResultRequest {
    /// Task identifier returned by the creation endpoint.
    task_id: String,
}

impl FileParseResultRequest {
    /// Create a result handle for `task_id`.
    pub fn new(task_id: impl Into<String>) -> Self {
        Self {
            task_id: task_id.into(),
        }
    }

    /// Borrow the task identifier used by result requests.
    pub fn task_id(&self) -> &str {
        &self.task_id
    }

    /// Fetch the current task result in `format_type` through `client`.
    pub async fn get_result_via(
        &self,
        client: &ZaiClient,
        format_type: FormatType,
    ) -> ZaiResult<FileParseResultResponse> {
        if self.task_id.trim().is_empty() {
            return Err(crate::ZaiError::ApiError {
                code: codes::SDK_VALIDATION,
                message: "task_id cannot be blank".to_string(),
            });
        }
        let route = crate::client::routes::FILES_PARSE_RESULT;
        let format_type = format_type.to_string();
        let url = client
            .endpoints()
            .resolve_route(route, &[&self.task_id, &format_type])?;
        trace!(format = %format_type, "Fetching file parser result");
        client
            .send_empty::<FileParseResultResponse>(route.method(), url)
            .await
    }

    /// Poll until the task succeeds, fails, or reaches `timeout_seconds`.
    ///
    /// `poll_interval_seconds` must be at least one. A provider failure and a
    /// local timeout are returned as errors rather than successful responses.
    pub async fn wait_for_result_via(
        &self,
        client: &ZaiClient,
        format_type: FormatType,
        timeout_seconds: u64,
        poll_interval_seconds: u64,
    ) -> ZaiResult<FileParseResultResponse> {
        if poll_interval_seconds == 0 {
            return Err(crate::client::error::ZaiError::ApiError {
                code: codes::SDK_VALIDATION,
                message: "poll_interval_seconds must be at least 1".to_string(),
            });
        }
        if timeout_seconds == 0 {
            return Err(crate::client::validation::invalid(
                "timeout_seconds must be at least 1",
            ));
        }
        debug!(
            timeout_seconds,
            poll_interval_seconds, "Polling file parser result"
        );
        let timeout = tokio::time::Duration::from_secs(timeout_seconds);
        let deadline = tokio::time::Instant::now()
            .checked_add(timeout)
            .ok_or_else(|| crate::client::error::ZaiError::ApiError {
                code: codes::SDK_VALIDATION,
                message: "timeout_seconds is too large".to_string(),
            })?;

        loop {
            trace!("Checking file parser result status");
            let result =
                tokio::time::timeout_at(deadline, self.get_result_via(client, format_type))
                    .await
                    .map_err(|_| polling_timeout())??;

            match result.status {
                ParserStatus::Succeeded => {
                    debug!("File parsing completed successfully");
                    return Ok(result);
                },
                ParserStatus::Failed => {
                    warn!("File parsing task reported failure");
                    return Err(parsing_failure(&result.message));
                },
                ParserStatus::Processing => {
                    let now = tokio::time::Instant::now();
                    let elapsed = timeout.saturating_sub(deadline.saturating_duration_since(now));
                    trace!(elapsed = ?elapsed, "File parser result still processing");
                    if now >= deadline {
                        warn!(
                            elapsed_seconds = elapsed.as_secs(),
                            timeout_seconds, "Polling timed out waiting for parsing result"
                        );
                        return Err(polling_timeout());
                    }
                    let interval = tokio::time::Duration::from_secs(poll_interval_seconds);
                    let wake_at = now.checked_add(interval).unwrap_or(deadline).min(deadline);
                    tokio::time::sleep_until(wake_at).await;
                    if wake_at == deadline {
                        return Err(polling_timeout());
                    }
                },
            }
        }
    }

    /// Fetch both text and download-link representations.
    ///
    /// This performs the two independent HTTP requests concurrently.
    ///
    pub async fn get_all_results_via(
        &self,
        client: &ZaiClient,
    ) -> ZaiResult<(FileParseResultResponse, FileParseResultResponse)> {
        let (text_result, download_result) = tokio::try_join!(
            self.get_result_via(client, FormatType::Text),
            self.get_result_via(client, FormatType::DownloadLink),
        )?;
        Ok((text_result, download_result))
    }
}

fn polling_timeout() -> crate::client::error::ZaiError {
    crate::client::error::ZaiError::ApiError {
        code: codes::SDK_TIMEOUT,
        message: "Timeout waiting for parsing result".to_string(),
    }
}

fn parsing_failure(message: &str) -> crate::client::error::ZaiError {
    crate::client::error::ZaiError::ApiError {
        code: codes::SDK_EXTERNAL_TOOL,
        message: format!(
            "Parsing failed: {}",
            crate::client::error::mask_sensitive_info(message)
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_failure_messages_are_sanitized() {
        let error = parsing_failure("Authorization: Bearer private-token-value");
        let rendered = error.to_string();
        assert!(!rendered.contains("private-token-value"));
        assert!(rendered.contains("[AUTH_REDACTED]"));
    }
}
