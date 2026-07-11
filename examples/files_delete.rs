use zai_rs::client::ZaiClient;
use zai_rs::file::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = ZaiClient::from_env()?;
    let file_id = std::env::args()
        .nth(1)
        .expect("usage: files_delete <file_id>");
    let resp = FileDeleteRequest::new(file_id).send_via(&client).await?;
    println!("{resp:#?}");
    Ok(())
}
