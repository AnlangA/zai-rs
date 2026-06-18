//! File parser demo with real API
//!
//! This example demonstrates how to use the file parser API with a real API
//! key.

use std::path::Path;

use zai_rs::tool::{
    file_parser_create::{FileParserCreateRequest, FileType, ToolType},
    file_parser_result::{FileParserResultRequest, FormatType},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var_os("RUST_LOG").is_some() {
        let _ = tracing_subscriber::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .try_init();
    }

    let api_key = std::env::var("ZHIPU_API_KEY")?;

    // Create test file
    let test_file_path = Path::new("data/demo_document.txt");
    let test_content = r#"# Sample Document
# Content:
This demonstrates the file parsing capabilities of the Zhipu AI API.
The parser should extract and return the text content.
"#;

    std::fs::write(test_file_path, test_content)?;

    // === Method 1: Basic file parsing with wait for result ===
    tracing::trace!("\n=== Method 1: File parsing with polling ===");

    let create_request = FileParserCreateRequest::new(
        api_key.clone(),
        test_file_path,
        ToolType::Lite,
        FileType::TXT,
    )?;

    tracing::trace!("Creating parsing task...");
    let create_response = create_request.send().await?;
    tracing::trace!("Task created: {}", create_response.task_id);

    // Wait for the result with polling
    let result_request = FileParserResultRequest::new(api_key.clone(), &create_response.task_id);
    tracing::trace!("Waiting for parsing result...");

    match result_request
        .wait_for_result(FormatType::Text, 1000, 3)
        .await
    {
        Ok(result_response) => {
            tracing::trace!("Parsing completed!");
            tracing::trace!("Status: {:?}", result_response.status);
            tracing::trace!("Message: {}", result_response.message);

            if let Some(content) = result_response.content() {
                tracing::trace!("Content length: {} characters", content.len());
                tracing::trace!("Preview:");
                tracing::trace!("{}", content.chars().take(500).collect::<String>());
                if content.len() > 500 {
                    tracing::trace!("... (truncated)");
                }
            }

            if let Some(download_url) = result_response.download_url() {
                tracing::trace!("Download URL: {}", download_url);
            }
        },
        Err(e) => {
            tracing::trace!("Error waiting for result: {}", e);
        },
    }

    // Cleanup
    if test_file_path.exists() {
        std::fs::remove_file(test_file_path)?;
        tracing::trace!("Cleaned up test file");
    }

    tracing::trace!("\nDemo completed successfully!");
    Ok(())
}
