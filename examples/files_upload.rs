use zai_rs::file::*;
use zai_rs::client::http::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let key = std::env::var("ZHIPU_API_KEY").expect("Please set ZHIPU_API_KEY env var");

    // Choose a local file to upload
    let path = std::env::args().nth(1).unwrap_or_else(|| "data/file_test.md".to_string());

    // purpose: choose one from batch/file-extract/code-interpreter/agent/voice-clone-input
    let purpose = FilePurpose::FileExtract;

    let client = FileUploadRequest::new(key, purpose, &path)
        // .with_file_name("custom_name.pdf")
        // .with_content_type("application/pdf")
        ;

    let resp = client.post().await?;
    let status = resp.status();
    if !status.is_success() {
        let txt = resp.text().await.unwrap_or_default();
        eprintln!("Request failed: {}\n{}", status, txt);
        return Ok(());
    }

    let body: FileObject = resp.json().await?;
    println!("Uploaded file: id={:?} filename={:?} bytes={:?} purpose={:?}",
        body.id, body.filename, body.bytes, body.purpose);

    Ok(())
}

