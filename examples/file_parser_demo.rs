//! Upload a document to the parser and wait for the asynchronous result.

use std::path::PathBuf;

use zai_rs::{
    client::ZaiClient,
    tool::{
        file_parser_create::{FileParseRequest, ToolType},
        file_parser_result::{FileParseResultRequest, FormatType},
    },
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .ok_or("usage: file_parser_demo <document>")?;

    let client = ZaiClient::from_env()?;
    let create_request = FileParseRequest::new_with_auto_type(&path, ToolType::Lite)?;
    let create_response = create_request.send_via(&client).await?;
    let task_id = create_response
        .task_id()
        .ok_or("file parser response did not include task_id")?;
    println!("submitted task {task_id}");

    let result = FileParseResultRequest::new(task_id)
        .wait_for_result_via(&client, FormatType::Text, 300, 2)
        .await?;
    println!("{result:#?}");
    Ok(())
}
