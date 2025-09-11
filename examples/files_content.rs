use zai_rs::file::*;
use zai_rs::client::http::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let key = std::env::var("ZHIPU_API_KEY").expect("Please set ZHIPU_API_KEY env var");
    let file_id = std::env::args().nth(1).unwrap_or_else(|| "1757560796_96cfc41fde21465a84ec796047020d81".to_string());

    let req = FileContentRequest::new(key, file_id.clone());
    let resp = req.get().await?;

    let status = resp.status();
    if !status.is_success() {
        let txt = resp.text().await.unwrap_or_default();
        eprintln!("Request failed: {}\n{}", status, txt);
        return Ok(());
    }

    let bytes = resp.bytes().await?;
    std::fs::create_dir_all("out").ok();
    let out_path = format!("out/{}_content.bin", file_id);
    std::fs::write(&out_path, &bytes)?;
    println!("Saved {} bytes to {}", bytes.len(), out_path);

    Ok(())
}

