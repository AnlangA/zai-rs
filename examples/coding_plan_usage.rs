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

use zai_rs::{ZaiResult, client::ZaiClient, usage::CodingPlanUsageRequest};

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

    // Build the shared ZaiClient. The Monitor family defaults to the official
    // endpoint; override it with `.endpoint(ApiFamily::Monitor, base)` for the
    // international endpoint.
    let client: ZaiResult<ZaiClient> = ZaiClient::builder(key).build();
    let client = client?;

    // Query the monitor endpoint. The raw response body is logged at `trace`
    // inside the transport decoder, so run with `RUST_LOG=trace` to see it.
    let resp = CodingPlanUsageRequest::new().send_via(&client).await?;

    // Pretty-print the normalized summary (also available via
    // `query_coding_plan_usage_summary` at the crate root).
    println!("{}", resp.summary());

    Ok(())
}
