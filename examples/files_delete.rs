use zai_rs::file::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let key = std::env::var("ZHIPU_API_KEY").expect("Please set ZHIPU_API_KEY env var");

    // file id to delete, pass as arg or hardcode for testing
    let file_id = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "1757561531_ec561569199641b3a5c556503a72cb79".to_string());

    let req = FileDeleteRequest::new(key, file_id);
    let resp = req.delete().await?;

    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        eprintln!("Request failed: {}\n{}", status, text);
        return Ok(());
    }

    let body: FileDeleteResponse = serde_json::from_str(&text)?;
    println!("{:#?}", body);

    Ok(())
}
