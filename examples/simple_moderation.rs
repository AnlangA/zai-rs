//! Simple content moderation example
//!
//! This example demonstrates basic usage of the content moderation API.

use zai_rs::model::moderation::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Get API key from environment
    let api_key =
        std::env::var("ZHIPU_API_KEY").expect("ZHIPU_API_KEY environment variable not set");

    // Text moderation example
    let text_content = "审核内容安全样例字符串。";

    let moderation = Moderation::new_text(text_content, api_key);
    let result = moderation.send().await?;

    println!("Moderation Result:");

    if let Some(id) = &result.id {
        println!("Task ID: {}", id);
    }
    if let Some(request_id) = &result.request_id {
        println!("Request ID: {}", request_id);
    }
    if let Some(created) = &result.created {
        println!("Created: {}", created);
    }

    if let Some(results) = &result.result_list {
        for (i, moderation_result) in results.iter().enumerate() {
            println!("Result {}: {:?}", i + 1, moderation_result);
        }
    }

    Ok(())
}
