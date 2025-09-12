use zai_rs::file::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let key = std::env::var("ZHIPU_API_KEY").expect("Please set ZHIPU_API_KEY env var");
    let file_id = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "1757561531_ec561569199641b3a5c556503a72cb79".to_string());

    // New: directly save to a file via send_to()
    let out_path = format!("out/{}_content.bin", file_id);
    let written = FileContentRequest::new(key, file_id.clone())
        .send_to(&out_path)
        .await?;

    println!("Saved {} bytes to {}", written, out_path);
    Ok(())
}
