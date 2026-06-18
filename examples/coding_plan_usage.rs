//! # Coding Plan Usage / Quota Query Example
//!
//! Queries the GLM Coding Plan remaining quota (余量) via the monitor API
//! `GET /api/monitor/usage/quota/limit` and prints the per-5-hour and weekly
//! windows.
//!
//! ## Prerequisites
//!
//! Set the `ZHIPU_API_KEY` environment variable with an API key bound to a GLM
//! Coding Plan subscription:
//! ```bash
//! export ZHIPU_API_KEY="your-api-key-here"
//! ```
//!
//! ## Running
//!
//! ```bash
//! # Trace level shows the raw monitor JSON response body.
//! RUST_LOG=trace cargo run --example coding_plan_usage
//! ```

use zai_rs::{client::http::HttpClientConfig, usage::CodingPlanUsageRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize structured logging. The SDK emits the raw request/response
    // bodies at `trace` level; API keys in the wire payload are masked
    // automatically when `mask_sensitive_data` is enabled (the default).
    let enable_logging = std::env::var_os("RUST_LOG").is_some();
    if enable_logging {
        let _ = tracing_subscriber::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .try_init();
    }

    let key = std::env::var("ZHIPU_API_KEY").expect("ZHIPU_API_KEY must be set");

    // `enable_logging(true)` turns on the request-body log emitted by the
    // transport. The raw response body is logged at `trace` inside
    // `parse_typed_response`, so run with `RUST_LOG=trace` to see it.
    let config = HttpClientConfig::builder().logging(enable_logging).build();

    // Query the official monitor endpoint.
    // Use `.with_monitor_base("https://api.z.ai/api/monitor")` for the
    // international endpoint.
    let resp = CodingPlanUsageRequest::new(key)
        .with_http_config(config)
        .send()
        .await?;

    // Pretty-print the normalized summary (also available via
    // `query_coding_plan_usage_summary` at the crate root).
    tracing::info!("{}", resp.summary());

    Ok(())
}
