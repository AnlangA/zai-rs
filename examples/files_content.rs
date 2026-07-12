//! Download a remote file to an explicit local path.

use std::path::PathBuf;

use zai_rs::{client::ZaiClient, file::FileContentRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let file_id = args
        .next()
        .ok_or("usage: files_content <file-id> <output-path>")?;
    let path = args
        .next()
        .map(PathBuf::from)
        .ok_or("usage: files_content <file-id> <output-path>")?;

    let client = ZaiClient::from_env()?;
    FileContentRequest::new(file_id)
        .send_to_via(&client, &path)
        .await?;
    println!("saved to {}", path.display());
    Ok(())
}
