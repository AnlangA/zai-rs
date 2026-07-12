//! List uploaded files used for document extraction.

use zai_rs::{
    client::ZaiClient,
    file::{FileListPurpose, FileListQuery, FileListRequest, FileListResponse},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = ZaiClient::from_env()?;
    let query = FileListQuery::new(FileListPurpose::Agent);
    let body: FileListResponse = FileListRequest::new(FileListPurpose::Agent)
        .with_query(query)
        .send_via(&client)
        .await?;
    println!("{body:#?}");
    Ok(())
}
