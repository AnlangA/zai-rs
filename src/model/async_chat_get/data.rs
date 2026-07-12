use super::response::AsyncTaskResult;
use crate::client::ZaiClient;

/// Request for retrieving any asynchronous task by its task identifier.
#[derive(Clone)]
pub struct AsyncTaskGetRequest {
    task_id: String,
}

impl std::fmt::Debug for AsyncTaskGetRequest {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AsyncTaskGetRequest")
            .field("task_id", &"[REDACTED]")
            .finish()
    }
}

impl AsyncTaskGetRequest {
    /// Create a new get-result request for the given task id.
    pub fn new(task_id: impl Into<String>) -> Self {
        Self {
            task_id: task_id.into(),
        }
    }

    /// Borrow the task identifier used in the result URL.
    pub fn task_id(&self) -> &str {
        &self.task_id
    }

    /// Validate that the task id is non-empty.
    pub fn validate(&self) -> crate::ZaiResult<()> {
        if self.task_id.trim().is_empty() {
            return Err(crate::client::error::ZaiError::ApiError {
                code: crate::client::error::codes::SDK_VALIDATION,
                message: "task_id must be non-empty".to_string(),
            });
        }
        Ok(())
    }

    /// Fetch the asynchronous task result via a [`ZaiClient`].
    pub async fn send_via(&self, client: &ZaiClient) -> crate::ZaiResult<AsyncTaskResult> {
        self.validate()?;
        let route = crate::client::routes::TASKS_GET;
        let url = client.endpoints().resolve_route(route, &[&self.task_id])?;
        client
            .send_empty::<AsyncTaskResult>(route.method(), url)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_validates_and_redacts_the_task_identifier() {
        assert!(AsyncTaskGetRequest::new(" ").validate().is_err());
        let request = AsyncTaskGetRequest::new("private-task-id");
        assert_eq!(request.task_id(), "private-task-id");
        assert!(!format!("{request:?}").contains("private-task-id"));
    }
}
