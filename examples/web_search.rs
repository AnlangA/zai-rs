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
    if std::env::var_os("RUST_LOG").is_some() {
        let _ = tracing_subscriber::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .try_init();
    }

    // Get API key from environment
    let api_key =
        std::env::var("ZHIPU_API_KEY").expect("ZHIPU_API_KEY environment variable must be set");

    tracing::trace!("=== Web Search Example ===\n");

    // Create a simple web search request
    let request = WebSearchRequest::new(
        api_key,
        "rust programming language".to_string(),
        SearchEngine::SearchStd,
    )
    .with_count(3) // Limit results for cleaner output
    .with_search_intent(true);

    tracing::trace!("Searching for 'rust programming language'...");

    match request.send().await {
        Ok(response) => {
            tracing::trace!("✓ Search successful!");
            tracing::trace!("Found {} results", response.result_count());

            if !response.intents().is_empty() {
                tracing::trace!("Detected intent: {}", response.intents()[0].intent);
            }

            // Show first result
            if let Some(first_result) = response.results().first() {
                tracing::trace!("\nFirst result:");
                tracing::trace!("  Title: {}", first_result.title);
                tracing::trace!("  URL: {}", first_result.link);
                tracing::trace!("  Source: {}", first_result.media);
            }
        },
        Err(e) => {
            tracing::trace!("✗ Search failed: {}", e);
        },
    }

    tracing::trace!("\n=== Example Complete ===");
    Ok(())
}
