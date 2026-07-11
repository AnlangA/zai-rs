use std::marker::PhantomData;
use std::sync::Arc;

use super::super::traits::*;
use crate::client::ZaiClient;
use crate::client::http::{HttpClientConfig, parse_typed_response, send_empty_request};

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
        let config = transport_config_from_client(client);
        let resp = send_empty_request(
            reqwest::Method::GET,
            url,
            client.secret().expose(),
            Arc::new(config),
        )
        .await?;
        parse_typed_response::<crate::model::chat_base_response::ChatCompletionResponse>(resp).await
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
