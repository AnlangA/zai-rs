use zai_rs::client::v2::ZaiClient;
use zai_rs::tool::{
    file_parser_create::{FileParserCreateRequest, FileType, ToolType},
    file_parser_result::{FileParserResultRequest, FormatType},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = ZaiClient::from_env()?;
    let test_file_path = std::env::temp_dir().join("zai_demo_document.txt");
    std::fs::write(&test_file_path, "Sample content")?;

    let create_request =
        FileParserCreateRequest::new(&test_file_path, ToolType::Lite, FileType::TXT)?;
    let create_response = create_request.send_via(&client).await?;
    println!("Task: {}", create_response.task_id);

    let result = FileParserResultRequest::new(&create_response.task_id)
        .get_result_via(&client, FormatType::Text)
        .await?;
    println!("{result:#?}");
    Ok(())
}
