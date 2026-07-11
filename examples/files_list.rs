use zai_rs::client::ZaiClient;
use zai_rs::file::{FileListQuery, FileListRequest, FileListResponse, FilePurpose};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = ZaiClient::from_env()?;
    let query = FileListQuery::new().with_purpose(FilePurpose::FileExtract);
    let body: FileListResponse = FileListRequest::new()
        .with_query(query)
        .send_via(&client)
        .await?;
    println!("{body:#?}");
    Ok(())
}
