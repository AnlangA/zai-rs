use std::marker::PhantomData;

use super::super::traits::*;
use crate::client::ZaiClient;

/// Retrieve the result of an asynchronous chat task by its task id (P05: routes
/// through [`ZaiClient`]).
pub struct AsyncChatGetRequest<N>
where
    N: ModelName + AsyncChat,
{
    task_id: String,
    _marker: PhantomData<N>,
}

impl<N> AsyncChatGetRequest<N>
where
    N: ModelName + AsyncChat,
{
    /// Create a new get-result request for the given task id.
    pub fn new(_model: N, task_id: String) -> Self {
        Self {
            task_id,
            _marker: PhantomData,
        }
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
    pub async fn send_via(
        &self,
        client: &ZaiClient,
    ) -> crate::ZaiResult<crate::model::chat_base_response::ChatCompletionResponse> {
        self.validate()?;
        let url = client.endpoints().resolve(
            crate::client::ApiFamily::PaasV4,
            &["async-result", &self.task_id],
        )?;
        client
            .send_empty::<crate::model::chat_base_response::ChatCompletionResponse>("GET", url)
            .await
    }
}
