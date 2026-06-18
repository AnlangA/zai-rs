//! Simple content moderation example
//!
//! This example demonstrates basic usage of the content moderation API.

use zai_rs::model::moderation::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var_os("RUST_LOG").is_some() {
        let _ = tracing_subscriber::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .try_init();
    }

    // Get API key from environment
    let api_key =
        std::env::var("ZHIPU_API_KEY").expect("ZHIPU_API_KEY environment variable not set");

    // Text moderation example
    let text_content = "审核内容安全样例字符串。";

    let moderation = Moderation::new_text(text_content, api_key);
    let result = moderation.send().await?;

    tracing::trace!("Moderation Result:");

    if let Some(id) = &result.id {
        tracing::trace!("Task ID: {}", id);
    }
    if let Some(request_id) = &result.request_id {
        tracing::trace!("Request ID: {}", request_id);
    }
    if let Some(created) = &result.created {
        tracing::trace!("Created: {}", created);
    }

    if let Some(results) = &result.result_list {
        for (i, moderation_result) in results.iter().enumerate() {
            tracing::trace!("Result {}: {:?}", i + 1, moderation_result);
        }
    }

    Ok(())
}
