//! Upload a local file for document extraction.

use std::path::PathBuf;

use zai_rs::{
    client::ZaiClient,
    file::{FileUploadPurpose, FileUploadRequest, FileUploadResponse},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .ok_or("usage: files_upload <local-file>")?;

    let client = ZaiClient::from_env()?;
    let body: FileUploadResponse = FileUploadRequest::new(FileUploadPurpose::Agent, path)
        .send_via(&client)
        .await?;
    println!("{body:#?}");
    Ok(())
}
