use zai_rs::client::ZaiClient;
use zai_rs::file::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = ZaiClient::from_env()?;
    let path = match std::env::args().nth(1) {
        Some(p) => p,
        None => {
            eprintln!("usage: files_upload <local-file>");
            std::process::exit(2);
        },
    };
    let body: FileObject = FileUploadRequest::new(FilePurpose::FileExtract, &path)
        .send_via(&client)
        .await?;
    println!("Uploaded: id={:?} filename={:?}", body.id, body.filename);
    Ok(())
}
