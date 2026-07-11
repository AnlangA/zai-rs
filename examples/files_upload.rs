use zai_rs::file::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let key = std::env::var("ZHIPU_API_KEY").expect("Please set ZHIPU_API_KEY env var");

    // Choose a local file to upload (passed as the first CLI argument).
    let path = match std::env::args().nth(1) {
        Some(p) => p,
        None => {
            eprintln!("usage: files_upload <local-file>");
            std::process::exit(2);
        },
    };

    // purpose: choose one from
    // batch/file-extract/code-interpreter/agent/voice-clone-input
    let purpose = FilePurpose::FileExtract;

    let client = FileUploadRequest::new(key, purpose, &path)
        // .with_file_name("custom_name.pdf")
        // .with_content_type("application/pdf")
        ;

    let body: FileObject = client.send().await?;
    println!(
        "Uploaded file: id={:?} filename={:?} bytes={:?} purpose={:?}",
        body.id, body.filename, body.bytes, body.purpose
    );

    Ok(())
}
