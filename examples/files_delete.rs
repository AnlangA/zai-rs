//! Delete a file by its provider-issued identifier.

use zai_rs::{client::ZaiClient, file::FileDeleteRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let file_id = std::env::args()
        .nth(1)
        .ok_or("usage: files_delete <file-id>")?;

    let client = ZaiClient::from_env()?;
    let resp = FileDeleteRequest::new(file_id).send_via(&client).await?;
    println!("{resp:#?}");
    Ok(())
}
