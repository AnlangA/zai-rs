//! File parser demo with real API
//!
//! This example demonstrates how to use the file parser API with a real API key.

use std::path::Path;
use zai_rs::tool::file_parser_create::{FileParserCreateRequest, FileType, ToolType};
use zai_rs::tool::file_parser_result::{FileParserResultRequest, FormatType};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
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
    println!("\n=== Method 1: File parsing with polling ===");

    let create_request = FileParserCreateRequest::new(
        api_key.clone(),
        test_file_path,
        ToolType::Lite,
        FileType::TXT,
    )?;

    println!("Creating parsing task...");
    let create_response = create_request.send().await?;
    println!("Task created: {}", create_response.task_id);

    // Wait for the result with polling
    let result_request = FileParserResultRequest::new(api_key.clone(), &create_response.task_id);
    println!("Waiting for parsing result...");

    match result_request
        .wait_for_result(FormatType::Text, 1000, 3)
        .await
    {
        Ok(result_response) => {
            println!("Parsing completed!");
            println!("Status: {:?}", result_response.status);
            println!("Message: {}", result_response.message);

            if let Some(content) = result_response.content() {
                println!("Content length: {} characters", content.len());
                println!("Preview:");
                println!("{}", content.chars().take(500).collect::<String>());
                if content.len() > 500 {
                    println!("... (truncated)");
                }
            }

            if let Some(download_url) = result_response.download_url() {
                println!("Download URL: {}", download_url);
            }
        }
        Err(e) => {
            println!("Error waiting for result: {}", e);
        }
    }

    // Cleanup
    if test_file_path.exists() {
        std::fs::remove_file(test_file_path)?;
        println!("Cleaned up test file");
    }

    println!("\nDemo completed successfully!");
    Ok(())
}
