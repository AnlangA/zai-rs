//! Request facade for the Coding Plan monitor operation.

use crate::client::ZaiClient;
use crate::{ZaiResult, usage::CodingPlanUsageResponse};

/// Coding Plan usage and quota query request.
///
/// This body-less request targets `GET /api/monitor/usage/quota/limit`.
/// Credentials, endpoint selection, and transport policy come from the
/// [`ZaiClient`] passed to [`Self::send_via`].
///
/// ```rust,no_run
/// use zai_rs::client::ZaiClient;
/// use zai_rs::usage::CodingPlanUsageRequest;
///
/// # async fn go(client: ZaiClient) -> zai_rs::ZaiResult<()> {
/// let response = CodingPlanUsageRequest::new().send_via(&client).await?;
/// if let Some(five_hour) = response.time_limit() {
///     tracing::info!("5h window: {}% used", five_hour.percentage);
/// }
/// # Ok(())
/// # }
/// ```
pub struct CodingPlanUsageRequest;

impl CodingPlanUsageRequest {
    /// Build a quota query whose runtime configuration comes from the client.
    pub fn new() -> Self {
        Self
    }

    /// Send the quota query and parse its typed envelope.
    ///
    /// # Errors
    ///
    /// Returns a validation, transport, provider, or response-decoding error
    /// reported by the shared client pipeline.
    pub async fn send_via(&self, client: &ZaiClient) -> ZaiResult<CodingPlanUsageResponse> {
        client
            .operation(crate::client::routes::USAGE_GET)
            .send_empty::<CodingPlanUsageResponse>()
            .await
    }
}

impl Default for CodingPlanUsageRequest {
    fn default() -> Self {
        Self::new()
    }
}
