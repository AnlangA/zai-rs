//! Print the normalized quota summary for a GLM Coding Plan subscription.

use zai_rs::{client::ZaiClient, usage::CodingPlanUsageRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var_os("RUST_LOG").is_some() {
        let _ = tracing_subscriber::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .try_init();
    }

    let client = ZaiClient::from_env()?;
    let resp = CodingPlanUsageRequest::new().send_via(&client).await?;
    println!("{}", resp.summary());

    Ok(())
}
