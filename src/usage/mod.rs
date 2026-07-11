//! # Coding Plan Usage / Quota Query
//!
//! Query the remaining quota and consumption statistics for the GLM Coding
//! Plan via the monitor API `GET /api/monitor/usage/quota/limit`.
//!
//! The Coding Plan applies two quota windows — a per-5-hour time window
//! (`TIME_LIMIT`) and a weekly tokens window (`TOKENS_LIMIT`) — and reports the
//! configured cap, consumed percentage, and next reset time for each. This
//! module surfaces them as strongly-typed [`CodingPlanUsageResponse`].
//!
//! Verified against the official `glm-plan-usage` plugin
//! (<https://docs.bigmodel.cn/cn/coding-plan/extension/usage-query-plugin>)
//! and the community CLI <https://github.com/JinHanAI/coding-plan-monitor>.
//!
//! ## Quick start
//!
//! ```rust,no_run
//! use zai_rs::client::ZaiClient;
//! use zai_rs::usage::CodingPlanUsageRequest;
//!
//! # async fn go(client: ZaiClient) -> zai_rs::ZaiResult<()> {
//! let resp = CodingPlanUsageRequest::new().send_via(&client).await?;
//!
//! let usage = resp.summary();
//!
//! if let Some(five_hour) = usage.time_limit() {
//!     tracing::info!(
//!         "5h window: {}/{} used ({} remaining, {}%), resets at {}",
//!         five_hour.used,
//!         five_hour.quota,
//!         five_hour.remaining,
//!         five_hour.percentage,
//!         five_hour
//!             .next_reset_at
//!             .as_ref()
//!             .map_or_else(|| "?".to_string(), |datetime| datetime.to_rfc3339())
//!     );
//! }
//! if let Some(weekly) = usage.tokens_limit() {
//!     tracing::info!(
//!         "weekly tokens: {} remaining of {}",
//!         weekly.remaining,
//!         weekly.quota
//!     );
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Switching to the international endpoint
//!
//! Override the monitor family base on the [`ZaiClient`] builder:
//!
//! ```rust,no_run
//! use zai_rs::client::{ApiFamily, ZaiClient};
//! use zai_rs::usage::CodingPlanUsageRequest;
//!
//! # async fn go() -> zai_rs::ZaiResult<()> {
//! static INTL: &str = "https://api.z.ai/api/monitor";
//! let client = ZaiClient::builder("your-api-key")
//!     .endpoint(ApiFamily::Monitor, INTL)
//!     .build()?;
//! let resp = CodingPlanUsageRequest::new().send_via(&client).await?;
//! # Ok(())
//! # }
//! ```

pub mod data;

pub use data::{
    CodingPlanQuotaKind, CodingPlanQuotaLimit, CodingPlanQuotaSummary, CodingPlanUsageData,
    CodingPlanUsageDetail, CodingPlanUsageRequest, CodingPlanUsageResponse, CodingPlanUsageSummary,
};

use crate::{ZaiResult, client::ZaiClient};

/// Query the Coding Plan usage endpoint via `client` and return the typed raw
/// response.
pub async fn query(client: &ZaiClient) -> ZaiResult<CodingPlanUsageResponse> {
    CodingPlanUsageRequest::new().send_via(client).await
}

/// Query the Coding Plan usage endpoint via `client` and return a normalized
/// summary.
pub async fn query_summary(client: &ZaiClient) -> ZaiResult<CodingPlanUsageSummary> {
    query(client).await.map(|response| response.summary())
}
