use zai_rs::client::v2::ZaiClient;
use zai_rs::file::FileContentRequest;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = ZaiClient::from_env()?;
    let file_id = std::env::args()
        .nth(1)
        .expect("usage: files_content <file_id>");
    let path = std::env::temp_dir().join("zai_file_content.bin");
    let p = path.to_str().unwrap();
    FileContentRequest::new(file_id)
        .send_to_via(&client, p)
        .await?;
    println!("saved to {p}");
    Ok(())
}
