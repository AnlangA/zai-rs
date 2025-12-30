//! Web Search API Example
//!
//! This example demonstrates basic usage of the web search API.
//!
//! # Usage
//!
//! ```bash
//! export ZHIPU_API_KEY="your_api_key_here"
//! cargo run --example web_search_example
//! ```

use zai_rs::tool::web_search::{SearchEngine, WebSearchRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logger
    let _ = env_logger::try_init();

    // Get API key from environment
    let api_key =
        std::env::var("ZHIPU_API_KEY").expect("ZHIPU_API_KEY environment variable must be set");

    println!("=== Web Search Example ===\n");

    // Create a simple web search request
    let request = WebSearchRequest::new(
        api_key,
        "rust programming language".to_string(),
        SearchEngine::SearchStd,
    )
    .with_count(3) // Limit results for cleaner output
    .with_search_intent(true);

    println!("Searching for 'rust programming language'...");

    match request.send().await {
        Ok(response) => {
            println!("✓ Search successful!");
            println!("Found {} results", response.result_count());

            if !response.intents().is_empty() {
                println!("Detected intent: {}", response.intents()[0].intent);
            }

            // Show first result
            if let Some(first_result) = response.results().first() {
                println!("\nFirst result:");
                println!("  Title: {}", first_result.title);
                println!("  URL: {}", first_result.link);
                println!("  Source: {}", first_result.media);
            }
        },
        Err(e) => {
            println!("✗ Search failed: {}", e);
        },
    }

    println!("\n=== Example Complete ===");
    Ok(())
}
